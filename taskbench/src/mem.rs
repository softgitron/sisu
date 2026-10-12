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
    let mut s = fs::read_to_string("/proc/self/status").map(|t| parse_status(&t)).unwrap_or_default();
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
    fs::read("/proc/self/maps").map(|b| b.iter().filter(|&&c| c == b'\n').count() as u64).unwrap_or(0)
}

/// Number of resident pages in `[addr, addr+len)` according to `mincore(2)`.
/// Precise per range, unlike process RSS, so the strategy tests use it.
pub fn resident_pages(addr: usize, len: usize) -> usize {
    let page = crate::stack::os::page_size();
    let start = addr & !(page - 1);
    let end = (addr + len + page - 1) & !(page - 1);
    let n = (end - start) / page;
    let mut vec = vec![0u8; n];
    // SAFETY: `vec` has one byte per page of the (page-aligned) range; mincore
    // only reads the page tables of our own mappings.
    let r = unsafe { libc::mincore(start as *mut libc::c_void, end - start, vec.as_mut_ptr()) };
    if r != 0 {
        return 0;
    }
    vec.iter().filter(|&&b| b & 1 != 0).count()
}

/// The `[start, end)` of the mapping in `/proc/self/maps` that contains `addr`.
pub fn mapping_containing(addr: usize) -> Option<(usize, usize)> {
    let maps = fs::read_to_string("/proc/self/maps").ok()?;
    maps.lines().find_map(|l| {
        let range = l.split_whitespace().next()?;
        let (a, b) = range.split_once('-')?;
        let (a, b) = (usize::from_str_radix(a, 16).ok()?, usize::from_str_radix(b, 16).ok()?);
        (a <= addr && addr < b).then_some((a, b))
    })
}

/// Reads an integer sysctl, e.g. `read_sysctl("vm/max_map_count")`.
pub fn read_sysctl(path: &str) -> Option<u64> {
    fs::read_to_string(format!("/proc/sys/{path}")).ok()?.split_whitespace().next()?.parse().ok()
}

/// `MemAvailable` from `/proc/meminfo`, in bytes.
pub fn mem_available() -> Option<u64> {
    let t = fs::read_to_string("/proc/meminfo").ok()?;
    kb_field(&t, "MemAvailable")
}

/// Reads file `v1_name` (cgroup v1, in the `controller` hierarchy) or `v2_name`
/// (cgroup v2) of the cgroup this process is in, falling back to the hierarchy's
/// root when the process's path is not visible (a cgroup namespace).
pub fn cgroup_file(controller: &str, v1_name: &str, v2_name: &str) -> Option<String> {
    let own = fs::read_to_string("/proc/self/cgroup").ok()?;
    let mut candidates = Vec::new();
    for line in own.lines() {
        // "hierarchy-id:controllers:path"; v2 has an empty controller list.
        let mut parts = line.splitn(3, ':');
        let (_, controllers, path) = (parts.next()?, parts.next()?, parts.next()?);
        if controllers.split(',').any(|c| c == controller) {
            candidates.push(format!("/sys/fs/cgroup/{controller}{path}/{v1_name}"));
            candidates.push(format!("/sys/fs/cgroup/{controller}/{v1_name}"));
        } else if controllers.is_empty() {
            candidates.push(format!("/sys/fs/cgroup{path}/{v2_name}"));
            candidates.push(format!("/sys/fs/cgroup/{v2_name}"));
        }
    }
    candidates.iter().find_map(|f| fs::read_to_string(f).ok()).map(|t| t.trim().to_string())
}

/// The memory cgroup's limit in bytes, `None` when unlimited or unreadable.
/// cgroup v1 reports "unlimited" as a huge number, v2 as `max`.
pub fn cgroup_memory_limit() -> Option<u64> {
    let v: u64 = cgroup_file("memory", "memory.limit_in_bytes", "memory.max")?.parse().ok()?;
    (v < 1 << 60).then_some(v)
}

/// The memory cgroup's usage in bytes, without inactive file pages (which the
/// kernel reclaims before the cgroup runs out).
pub fn cgroup_memory_usage() -> Option<u64> {
    let usage: u64 = cgroup_file("memory", "memory.usage_in_bytes", "memory.current")?.parse().ok()?;
    let stat = cgroup_file("memory", "memory.stat", "memory.stat").unwrap_or_default();
    let inactive = stat
        .lines()
        .find_map(|l| l.strip_prefix("total_inactive_file ").or_else(|| l.strip_prefix("inactive_file ")))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    Some(usage.saturating_sub(inactive))
}

/// Bytes that can still be allocated before memory runs out: `MemAvailable`,
/// lowered to the memory cgroup's headroom when the cgroup is the tighter limit.
pub fn mem_headroom() -> Option<u64> {
    let cgroup = cgroup_memory_limit().map(|limit| limit.saturating_sub(cgroup_memory_usage().unwrap_or(0)));
    match (mem_available(), cgroup) {
        (Some(a), Some(c)) => Some(a.min(c)),
        (a, c) => a.or(c),
    }
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
        let headroom = mem_headroom().expect("MemAvailable is always readable");
        assert!(headroom <= mem_available().unwrap());
        if let Some(limit) = cgroup_memory_limit() {
            assert!(headroom <= limit);
        }
        assert_eq!(per_item(100, 300, 4), 50.0);
    }
}
