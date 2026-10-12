//! Integration smoke tests: every benchmark, through the binary, at tiny scale.
//!
//! Each run writes a CSV that is parsed back and checked for the schema's column
//! count and for the metrics the report needs. Running the binary (rather than
//! the library) keeps each benchmark in its own process, as on a real run: a
//! crash on stack exhaustion fails the test instead of the test harness.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_taskbench");

fn tmp_csv(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("taskbench-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{name}.csv"));
    let _ = std::fs::remove_file(&p);
    p
}

/// Splits one CSV line (RFC 4180 quoting).
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

type Rows = Vec<HashMap<String, String>>;

/// Runs `taskbench <args> --csv <tmp> --quiet --threads 2` and returns the rows.
fn run(name: &str, args: &[&str]) -> Rows {
    let csv = tmp_csv(name);
    let out = Command::new(BIN)
        .args(args)
        .args(["--csv", csv.to_str().unwrap(), "--quiet", "--threads", "2"])
        .output()
        .expect("run taskbench");
    assert!(
        out.status.success(),
        "taskbench {args:?} failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&csv).expect("csv written");
    let mut lines = text.lines();
    let header = split_csv(lines.next().expect("header"));
    assert_eq!(header.join(","), taskbench::csv::HEADER);
    lines
        .map(|l| {
            let f = split_csv(l);
            assert_eq!(f.len(), header.len(), "bad arity: {l}");
            header.iter().cloned().zip(f).collect()
        })
        .collect()
}

fn metric<'a>(rows: &'a Rows, name: &str) -> &'a HashMap<String, String> {
    rows.iter().find(|r| r["metric"] == name).unwrap_or_else(|| panic!("no metric {name} in {rows:?}"))
}

fn value(rows: &Rows, name: &str) -> f64 {
    metric(rows, name)["value"].parse().unwrap_or_else(|_| panic!("{name} has no value"))
}

const STRATEGIES: &[&str] = &["A", "B", "C", "D", "E", "tokio"];

#[test]
fn spawn_join() {
    for s in STRATEGIES.iter().chain(&["os-threads"]) {
        let rows = run(&format!("sj-{s}"), &["spawn-join", "--strategy", s, "--rounds", "300", "--warmup", "30"]);
        assert!(value(&rows, "spawn_join_p50") > 0.0, "{s}");
        assert!(value(&rows, "spawn_join_p99") >= value(&rows, "spawn_join_p50"));
        assert_eq!(value(&rows, "spawn_join_samples"), 300.0);
        assert_eq!(metric(&rows, "spawn_join_p50")["strategy"], if *s == "D" { "D(A)" } else { s });
    }
    let rows =
        run("sj-batch", &["spawn-join", "--strategy", "B", "--rounds", "100", "--batch", "8", "--scope-per-round"]);
    assert_eq!(metric(&rows, "spawn_join_p50")["tasks"], "800");
}

#[test]
fn idle_memory_and_decommit() {
    let mut excursion = HashMap::new();
    for s in STRATEGIES {
        let rows = run(&format!("im-{s}"), &["idle-memory", "--strategy", s, "--tasks", "300", "--settle-ms", "20"]);
        assert_eq!(value(&rows, "tasks_created"), 300.0, "{s}");
        assert!(value(&rows, "total_per_task_shallow") > 0.0, "{s}");
        excursion.insert(*s, value(&rows, "total_per_task_excursion"));
    }
    // Commit creep: the 32 KiB excursion stays resident under B, D returns it.
    assert!(excursion["B"] > 30_000.0, "{excursion:?}");
    assert!(excursion["D"] < excursion["B"] / 2.0, "{excursion:?}");
    assert!(excursion["E"] < 4_000.0, "{excursion:?}");
    let rows = run("im-db", &["idle-memory", "--strategy", "D", "--base", "B", "--tasks", "200", "--settle-ms", "20"]);
    assert_eq!(metric(&rows, "tasks_created")["strategy"], "D(B)");
}

#[test]
fn idle_memory_guard_turns_exhaustion_into_a_result() {
    // A guard no machine can satisfy: spawning stops at the first check (every
    // 4096 tasks) and the run is a failed result rather than an OOM kill.
    let args = ["idle-memory", "--strategy", "B", "--tasks", "5000", "--settle-ms", "20", "--mem-guard", "1000000000G"];
    let rows = run("im-guard", &args);
    let created = metric(&rows, "tasks_created");
    assert_eq!(created["status"], "failed", "{created:?}");
    assert_eq!(value(&rows, "tasks_created"), 4096.0);
    assert!(created["note"].contains("memory headroom below"), "{created:?}");
}

#[test]
fn max_tasks() {
    for s in ["A", "B", "E", "tokio"] {
        let rows = run(&format!("mt-{s}"), &["max-tasks", "--strategy", s, "--tasks", "2000"]);
        assert_eq!(value(&rows, "max_tasks"), 2000.0, "{s}");
        assert_eq!(metric(&rows, "limit")["note"], "cap");
    }
    let rows = run("mt-os", &["max-tasks", "--strategy", "os-threads", "--tasks", "200"]);
    assert_eq!(value(&rows, "max_tasks"), 200.0);
}

#[test]
fn echo_and_idle_connections() {
    for s in ["B", "A", "E", "tokio"] {
        let rows = run(
            &format!("echo-{s}"),
            &["echo", "--strategy", s, "--connections", "16", "--duration", "0.3", "--warmup-secs", "0.1"],
        );
        assert!(value(&rows, "echo_requests") > 0.0, "{s}");
        assert!(value(&rows, "echo_rtt_p999") >= value(&rows, "echo_rtt_p50"));
        let rows = run(
            &format!("idle-{s}"),
            &["idle-connections", "--strategy", s, "--connections", "64", "--settle-ms", "20"],
        );
        assert!(value(&rows, "rss_per_connection").is_finite(), "{s}");
    }
}

#[test]
fn deep_recursion_is_clean_everywhere() {
    for s in ["A", "B", "C", "D", "E", "tokio"] {
        let rows = run(
            &format!("dr-{s}"),
            &["deep-recursion", "--strategy", s, "--parse-depth", "5000", "--naive-depth", "1000000"],
        );
        for case in ["parse_outcome", "naive_outcome"] {
            let r = metric(&rows, case);
            assert_eq!(r["status"], "ok", "{s} {case}: {r:?}");
            assert_eq!(r["value"], "1", "{s} {case}: {r:?}");
        }
        assert!(metric(&rows, "naive_outcome")["note"].starts_with("stack_exhausted"), "{s}: 1M deep must exhaust");
        assert_eq!(value(&rows, "runtime_healthy_after"), 1.0);
        let leaked = value(&rows, "naive_leaked_drop_guards");
        match s {
            // Guard-page traps discard frames; probes unwind them.
            "A" | "D" => assert!(leaked > 0.0, "{s}"),
            _ => assert_eq!(leaked, 0.0, "{s}"),
        }
    }
    let rows = run(
        "dr-small",
        &[
            "deep-recursion",
            "--strategy",
            "A",
            "--reservation",
            "128K",
            "--parse-depth",
            "10000",
            "--naive-depth",
            "1000",
        ],
    );
    assert!(metric(&rows, "parse_outcome")["note"].starts_with("stack_exhausted"));
    assert!(metric(&rows, "naive_outcome")["note"].starts_with("completed"));
}

#[test]
fn starvation() {
    for s in ["B", "E", "tokio", "os-threads"] {
        let rows = run(
            &format!("st-{s}"),
            &["starvation", "--strategy", s, "--duration", "0.3", "--baseline-secs", "0.1", "--tasks", "4"],
        );
        assert!(value(&rows, "probe_loaded_late_samples") >= 1.0, "{s}");
        assert!(value(&rows, "compute_iterations_per_sec") > 0.0, "{s}");
    }
    // No yield points and more compute tasks than workers: the probe starves.
    let rows = run(
        "st-none",
        &[
            "starvation",
            "--strategy",
            "B",
            "--duration",
            "0.3",
            "--baseline-secs",
            "0.1",
            "--tasks",
            "4",
            "--yield-every",
            "0",
        ],
    );
    assert!(value(&rows, "probe_loaded_late_p99") > 100_000_000.0);
}

#[test]
fn probe_overhead() {
    for s in ["B", "A", "E"] {
        let rows = run(
            &format!("po-{s}"),
            &[
                "probe-overhead",
                "--strategy",
                s,
                "--reps",
                "1",
                "--fib-n",
                "18",
                "--calls",
                "20000",
                "--tree-depth",
                "6",
            ],
        );
        for w in ["fib", "calls", "tree"] {
            for v in ["split", "arg", "tls"] {
                assert!(value(&rows, &format!("{w}_{v}_probe_overhead")).is_finite(), "{s} {w} {v}");
            }
        }
    }
    let rows = run("po-tokio", &["probe-overhead", "--strategy", "tokio"]);
    assert_eq!(metric(&rows, "probe_overhead")["status"], "skipped");
}

#[test]
fn cancel_uring() {
    let rows = run("cu-pipe", &["cancel-uring", "--strategy", "B", "--tasks", "256", "--concurrency", "64"]);
    let r = metric(
        &rows,
        if rows.iter().any(|r| r["metric"] == "buffer_writes_after_return") {
            "buffer_writes_after_return"
        } else {
            "cancel_uring"
        },
    );
    if r["status"] == "skipped" {
        eprintln!("io_uring unavailable here: {}", r["note"]);
        return;
    }
    assert_eq!(r["status"], "ok", "{r:?}");
    assert_eq!(r["value"], "0");
    assert_eq!(value(&rows, "reads"), 256.0);
    for (name, s) in [("cu-a", "A"), ("cu-c", "C"), ("cu-d", "D")] {
        let rows = run(name, &["cancel-uring", "--strategy", s, "--tasks", "128", "--concurrency", "64"]);
        assert_eq!(metric(&rows, "buffer_writes_after_return")["value"], "0", "{s}");
    }
    let rows = run(
        "cu-file",
        &[
            "cancel-uring",
            "--strategy",
            "B",
            "--tasks",
            "256",
            "--concurrency",
            "64",
            "--source",
            "file",
            "--file-bytes",
            "4M",
        ],
    );
    let r = metric(
        &rows,
        if rows.iter().any(|r| r["metric"] == "buffer_writes_after_return") {
            "buffer_writes_after_return"
        } else {
            "cancel_uring"
        },
    );
    if r["status"] != "skipped" {
        assert_eq!(r["value"], "0", "{r:?}");
    }
    // Negative control: the checker must catch writes after an early return.
    let rows = run(
        "cu-neg",
        &["cancel-uring", "--strategy", "B", "--tasks", "256", "--concurrency", "64", "--break-contract"],
    );
    let r = metric(&rows, "buffer_writes_after_return");
    assert_eq!(r["status"], "ok", "negative control did not detect violations: {r:?}");
    assert!(value(&rows, "buffer_writes_after_return") > 0.0);
    let rows = run("cu-tokio", &["cancel-uring", "--strategy", "tokio"]);
    assert_eq!(metric(&rows, "cancel_uring")["status"], "skipped");
}

#[test]
fn machine_spec_and_cli() {
    let rows = run("ms", &["machine-spec"]);
    assert!(metric(&rows, "sysctl.vm.max_map_count")["note"].parse::<u64>().is_ok());
    assert!(!metric(&rows, "cpu_model")["note"].is_empty());
    let rows =
        run("ms-flag", &["spawn-join", "--rounds", "10", "--warmup", "0", "--machine-spec", "--machine-id", "box-1"]);
    assert!(rows.iter().any(|r| r["benchmark"] == "machine-spec"));
    assert!(rows.iter().all(|r| r["machine_id"] == "box-1"));
    let out = Command::new(BIN).args(["spawn-join", "--roundz", "1"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let out = Command::new(BIN).arg("help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("cancel-uring"));
}
