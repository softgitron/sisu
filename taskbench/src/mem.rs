//! Process memory measurement from `/proc/self/status`, `/proc/self/smaps_rollup`
//! and `/proc/self/maps`.
//!
//! "Memory per idle task" in the bet-3 table is resident memory, and on Linux the
//! page tables that map task stacks are real memory that `VmRSS` does not include,
//! so every snapshot records `VmPTE` separately. Reservation size changes the
//! page-table cost (a touched page per 1 MiB reservation needs its own page-table
//! page per 2 MiB of address space), so the per-task figure the report compares
//! with Go is `RSS + PTE`.

use std::fs;

/// One reading of the process's memory counters, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemSnapshot {
    pub vm_rss: u64,
    pub rss_anon: u64,
    pub rss_file: u64,
    pub vm_pte: u64,
    pub vm_size: u64,
    pub threads: u64,
    /// `Pss` from smaps_rollup, when readable.
    pub pss: u64,
    /// Number of VMAs (lines in `/proc/self/maps`), when requested.
    pub vmas: u64,
}

impl MemSnapshot {
    /// Resident plus page-table bytes: the figure compared across implementations.
    pub fn rss_plus_pte(&self) -> u64 {
        self.vm_rss + self.vm_pte
    }
}

fn kb_field(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?.strip_prefix(':')?;
        let mut it = rest.split_whitespace();
        let v: u64 = it.next()?.parse().ok()?;
        match it.next() {
            Some("kB") => Some(v * 1024),
            _ => Some(v),
        }
    })
}

/// Parses the fields of interest from the text of `/proc/<pid>/status`.
pub fn parse_status(text: &str) -> MemSnapshot {
    MemSnapshot {
        vm_rss: kb_field(text, "VmRSS").unwrap_or(0),
        rss_anon: kb_field(text, "RssAnon").unwrap_or(0),
        rss_file: kb_field(text, "RssFile").unwrap_or(0),
        vm_pte: kb_field(text, "VmPTE").unwrap_or(0),
        vm_size: kb_field(text, "VmSize").unwrap_or(0),
        threads: kb_field(text, "Threads").unwrap_or(0),
        pss: 0,
        vmas: 0,
    }
}

/// Reads the current process's counters. `with_vmas` also counts VMAs, which
/// reads all of `/proc/self/maps` and is slow with hundreds of thousands of them.
pub fn snapshot(with_vmas: bool) -> MemSnapshot {
    let mut s = fs::read_to_string("/proc/self/status")
        .map(|t| parse_status(&t))
        .unwrap_or_default();
    if let Ok(t) = fs::read_to_string("/proc/self/smaps_rollup") {
        s.pss = kb_field(&t, "Pss").unwrap_or(0);
    }
    if with_vmas {
        s.vmas = vma_count();
    }
    s
}

/// Number of memory mappings of this process (compare with `vm.max_map_count`).
pub fn vma_count() -> u64 {
    fs::read("/proc/self/maps")
        .map(|b| b.iter().filter(|&&c| c == b'\n').count() as u64)
        .unwrap_or(0)
}

/// Reads an integer sysctl, e.g. `read_sysctl("vm/max_map_count")`.
pub fn read_sysctl(path: &str) -> Option<u64> {
    fs::read_to_string(format!("/proc/sys/{path}"))
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// `MemAvailable` from `/proc/meminfo`, in bytes.
pub fn mem_available() -> Option<u64> {
    let t = fs::read_to_string("/proc/meminfo").ok()?;
    kb_field(&t, "MemAvailable")
}

/// `MemTotal` from `/proc/meminfo`, in bytes.
pub fn mem_total() -> Option<u64> {
    let t = fs::read_to_string("/proc/meminfo").ok()?;
    kb_field(&t, "MemTotal")
}

/// Per-item difference `(after - before) / n` in bytes, signed.
pub fn per_item(before: u64, after: u64, n: u64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    (after as f64 - before as f64) / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status() {
        let text = "Name:\ttaskbench\nVmSize:\t  100 kB\nVmRSS:\t    8 kB\nRssAnon:\t 4 kB\nRssFile:\t 4 kB\nVmPTE:\t 2 kB\nThreads:\t5\n";
        let s = parse_status(text);
        assert_eq!(s.vm_rss, 8 * 1024);
        assert_eq!(s.rss_anon, 4 * 1024);
        assert_eq!(s.vm_pte, 2 * 1024);
        assert_eq!(s.vm_size, 100 * 1024);
        assert_eq!(s.threads, 5);
        assert_eq!(s.rss_plus_pte(), 10 * 1024);
    }

    #[test]
    fn reads_self() {
        let s = snapshot(true);
        assert!(s.vm_rss > 0);
        assert!(s.vmas > 0);
        assert!(read_sysctl("vm/max_map_count").is_some());
        assert_eq!(per_item(100, 300, 4), 50.0);
    }
}
