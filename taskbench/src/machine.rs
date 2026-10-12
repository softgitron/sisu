//! Machine specification recording (`--machine-spec` and `taskbench machine-spec`).
//!
//! Records what the bet-3 thresholds depend on: kernel, CPU, memory, and the
//! sysctls and resource limits that cap task counts (VMA count, threads, file
//! descriptors, overcommit, transparent huge pages).

use std::fs;

/// The machine identifier written into every CSV row: `--machine-id`, else the
/// host name.
pub fn default_machine_id() -> String {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_trim(path: &str) -> String {
    fs::read_to_string(path).map(|s| s.trim().replace('\n', " ")).unwrap_or_else(|e| format!("unreadable: {e}"))
}

fn uname() -> (String, String, String) {
    // SAFETY: `utsname` is plain old data and `uname` only writes into it.
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    // SAFETY: `u` is a valid, writable utsname.
    if unsafe { libc::uname(&mut u) } != 0 {
        return ("?".into(), "?".into(), "?".into());
    }
    let s = |f: &[libc::c_char]| {
        let bytes: Vec<u8> = f.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    (s(&u.sysname), s(&u.release), s(&u.version))
}

fn rlimit(res: libc::__rlimit_resource_t) -> String {
    let mut r = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: `r` is a valid out-parameter.
    if unsafe { libc::getrlimit(res, &mut r) } != 0 {
        return "?".into();
    }
    let f = |v: libc::rlim_t| {
        if v == libc::RLIM_INFINITY { "unlimited".to_string() } else { v.to_string() }
    };
    format!("{}/{}", f(r.rlim_cur), f(r.rlim_max))
}

/// Key/value pairs describing this machine.
pub fn collect() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let (sys, rel, ver) = uname();
    out.push(("uname".into(), format!("{sys} {rel} {ver}")));
    out.push(("hostname".into(), default_machine_id()));
    let os = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())))
        .unwrap_or_else(|| "?".into());
    out.push(("os".into(), os));
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = cpuinfo
        .lines()
        .find_map(|l| l.strip_prefix("model name").map(|r| r.trim_start_matches([' ', '\t', ':']).to_string()))
        .unwrap_or_else(|| "?".into());
    out.push(("cpu_model".into(), model));
    let online = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    out.push(("cpus_available".into(), online.to_string()));
    out.push(("cpus_online".into(), read_trim("/sys/devices/system/cpu/online")));
    out.push(("cpu_governor".into(), read_trim("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")));
    out.push(("mem_total_bytes".into(), crate::mem::mem_total().map(|v| v.to_string()).unwrap_or_else(|| "?".into())));
    let swap = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("SwapTotal")).map(|l| l.to_string()))
        .unwrap_or_default();
    out.push(("swap".into(), swap.split_whitespace().skip(1).collect::<Vec<_>>().join(" ")));
    // SAFETY: sysconf has no preconditions.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    out.push(("page_size".into(), page.to_string()));
    for key in [
        "vm/max_map_count",
        "vm/overcommit_memory",
        "vm/overcommit_ratio",
        "kernel/threads-max",
        "kernel/pid_max",
        "kernel/io_uring_disabled",
        "fs/nr_open",
        "fs/file-max",
        "net/core/somaxconn",
        "net/ipv4/ip_local_port_range",
        "net/ipv4/tcp_max_syn_backlog",
    ] {
        out.push((format!("sysctl.{}", key.replace('/', ".")), read_trim(&format!("/proc/sys/{key}"))));
    }
    out.push(("thp_enabled".into(), read_trim("/sys/kernel/mm/transparent_hugepage/enabled")));
    out.push(("thp_defrag".into(), read_trim("/sys/kernel/mm/transparent_hugepage/defrag")));
    out.push(("cpu_mitigations".into(), read_trim("/sys/devices/system/cpu/vulnerabilities/spectre_v2")));
    out.push(("rlimit_nofile".into(), rlimit(libc::RLIMIT_NOFILE)));
    out.push(("rlimit_nproc".into(), rlimit(libc::RLIMIT_NPROC)));
    out.push(("rlimit_stack".into(), rlimit(libc::RLIMIT_STACK)));
    out.push(("rlimit_as".into(), rlimit(libc::RLIMIT_AS)));
    out.push(("rlimit_memlock".into(), rlimit(libc::RLIMIT_MEMLOCK)));
    let cgroup = |controller: &str, v1: &str, v2: &str| {
        crate::mem::cgroup_file(controller, v1, v2).unwrap_or_else(|| "unreadable".into())
    };
    let memory_max = match crate::mem::cgroup_memory_limit() {
        Some(limit) => limit.to_string(),
        None => cgroup("memory", "memory.limit_in_bytes", "memory.max"),
    };
    out.push(("cgroup_memory_max".into(), memory_max));
    out.push(("cgroup_pids_max".into(), cgroup("pids", "pids.max", "pids.max")));
    out.push(("harness_version".into(), env!("CARGO_PKG_VERSION").to_string()));
    out
}

/// Raises the soft `RLIMIT_NOFILE` to the hard limit, as the Go runtime (1.19+)
/// and the JVM (`-XX:+MaxFDLimit`) do by default. Returns the new soft limit.
pub fn raise_nofile_limit() -> u64 {
    let mut r = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: valid out-parameter.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut r) } != 0 {
        return 0;
    }
    if r.rlim_cur < r.rlim_max {
        let want = libc::rlimit { rlim_cur: r.rlim_max, rlim_max: r.rlim_max };
        // SAFETY: valid in-parameter; lowering or keeping the hard limit is always allowed.
        if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &want) } == 0 {
            return want.rlim_cur;
        }
    }
    r.rlim_cur
}

#[cfg(test)]
mod tests {
    #[test]
    fn collects_core_keys() {
        let spec = super::collect();
        for key in ["uname", "cpu_model", "mem_total_bytes", "sysctl.vm.max_map_count", "rlimit_nofile"] {
            assert!(spec.iter().any(|(k, _)| k == key), "missing {key}");
        }
    }
}
