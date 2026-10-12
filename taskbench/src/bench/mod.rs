//! The benchmarks. Each one takes its scale from the command line, defaults to
//! a size that runs on a development machine in well under a minute, prints a
//! summary and writes rows of the shared CSV schema ([`crate::csv`]).
//!
//! | Subcommand | bet-3 row it measures |
//! |---|---|
//! | `spawn-join` | spawn-plus-join median and p99 (threshold: median within 3x Go with pooled stacks) |
//! | `idle-memory` | memory per idle task after shallow use and after a 32 KiB excursion (within 4x Go) |
//! | `max-tasks` | tasks creatable under the current sysctls, and which limit stops them |
//! | `echo` | 10k-connection echo p50/p99/p99.9 and throughput (p99 within 2x Go) |
//! | `idle-connections` | resident memory per idle connection (100k on the full run) |
//! | `deep-recursion` | clean "stack exhausted" on a 10k-deep parse and a 1M-deep recursion |
//! | `starvation` | latency-probe p99 under N compute-bound tasks (no worse than Go, under 10 ms) |
//! | `probe-overhead` | cost of software stack probes on compute loops (at most 3 %) |
//! | `cancel-uring` | io_uring reads into stack buffers cancelled in flight (amendment A1, D-058) |

pub mod cancel_uring;
pub mod deep_recursion;
pub mod echo;
pub mod idle_memory;
pub mod max_tasks;
pub mod probe_overhead;
pub mod spawn_join;
pub mod starvation;
pub mod work;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::csv::{CsvWriter, Row, Status, utc_timestamp};
use crate::rt::Runtime;
use crate::stack::{StrategyConfig, StrategyId};
use crate::units;

/// Which implementation runs the workload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    /// This crate's runtime with one of strategies A-E.
    Taskbench(StrategyConfig),
    /// tokio's multi-threaded runtime: the stackless reference control.
    Tokio,
    /// One OS thread per task (reference for spawn-join, idle-memory and max-tasks).
    OsThreads,
}

impl Impl {
    /// The CSV `implementation` column.
    pub fn implementation(&self) -> &'static str {
        match self {
            Impl::Taskbench(_) => "taskbench",
            Impl::Tokio => "tokio",
            Impl::OsThreads => "os-threads",
        }
    }

    /// The CSV `strategy` column.
    pub fn strategy_label(&self) -> String {
        match self {
            Impl::Taskbench(c) => c.label(),
            Impl::Tokio => "tokio".into(),
            Impl::OsThreads => "os-threads".into(),
        }
    }

    pub fn config(&self) -> Option<&StrategyConfig> {
        match self {
            Impl::Taskbench(c) => Some(c),
            _ => None,
        }
    }

    pub fn is_stackless(&self) -> bool {
        matches!(self, Impl::Taskbench(c) if c.id == StrategyId::E)
    }
}

/// Parsed command-line options.
#[derive(Clone, Debug)]
pub struct Args {
    pub command: String,
    pub imp: Impl,
    pub threads: usize,
    pub tasks: Option<u64>,
    pub csv: Option<PathBuf>,
    pub machine_spec: bool,
    pub machine_id: String,
    pub worker_stack: usize,
    pub quiet: bool,
    /// Benchmark-specific options, by name without the leading dashes.
    pub opts: BTreeMap<String, String>,
}

impl Args {
    /// Defaults for `command` (used by tests and by the CLI before parsing).
    pub fn new(command: &str) -> Args {
        Args {
            command: command.to_string(),
            imp: Impl::Taskbench(StrategyConfig::new(StrategyId::B)),
            threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            tasks: None,
            csv: None,
            machine_spec: false,
            machine_id: crate::machine::default_machine_id(),
            worker_stack: crate::rt::DEFAULT_WORKER_STACK,
            quiet: false,
            opts: BTreeMap::new(),
        }
    }

    pub fn opt(&self, key: &str) -> Option<&str> {
        self.opts.get(key).map(|s| s.as_str())
    }

    pub fn flag(&self, key: &str) -> bool {
        self.opts.get(key).map(|v| v != "false" && v != "0").unwrap_or(false)
    }

    pub fn count(&self, key: &str, default: u64) -> Result<u64, String> {
        self.opt(key).map(units::parse_count).transpose().map(|v| v.unwrap_or(default))
    }

    pub fn size(&self, key: &str, default: usize) -> Result<usize, String> {
        self.opt(key).map(units::parse_size).transpose().map(|v| v.unwrap_or(default))
    }

    pub fn secs(&self, key: &str, default: f64) -> Result<Duration, String> {
        self.opt(key)
            .map(units::parse_secs)
            .transpose()
            .map(|v| v.unwrap_or(Duration::from_secs_f64(default)))
    }

    /// `--tasks`, else the benchmark's own option, else its default.
    pub fn tasks_or(&self, alias: &str, default: u64) -> Result<u64, String> {
        match self.tasks {
            Some(t) => Ok(t),
            None => self.count(alias, default),
        }
    }
}

/// What a benchmark run has: options, CSV output, run id.
pub struct BenchCtx {
    pub args: Args,
    pub run_id: String,
    csv: Option<CsvWriter>,
    /// Every row emitted, for tests and the final status.
    pub rows: Vec<Row>,
}

impl BenchCtx {
    pub fn new(args: Args) -> Result<BenchCtx, String> {
        let csv = match &args.csv {
            Some(p) => Some(CsvWriter::open_append(p).map_err(|e| format!("cannot open {}: {e}", p.display()))?),
            None => None,
        };
        let run_id = format!("{}-{}", utc_timestamp().replace([':', '-'], ""), std::process::id());
        Ok(BenchCtx { args, run_id, csv, rows: Vec::new() })
    }

    /// A row with the common columns filled in.
    pub fn row(&self, metric: &str, value: Option<f64>, unit: &str) -> Row {
        let a = &self.args;
        let (reservation, stack) = match a.imp.config() {
            Some(c) => (
                match c.id {
                    StrategyId::A | StrategyId::B | StrategyId::D => Some(c.reservation as u64),
                    _ => None,
                },
                c.stack_bytes().map(|v| v as u64),
            ),
            None => (None, None),
        };
        Row {
            run_id: self.run_id.clone(),
            timestamp_utc: utc_timestamp(),
            machine_id: a.machine_id.clone(),
            implementation: a.imp.implementation().into(),
            benchmark: a.command.clone(),
            strategy: a.imp.strategy_label(),
            reservation_bytes: reservation,
            stack_bytes: stack,
            threads: Some(a.threads as u64),
            tasks: a.tasks,
            params: String::new(),
            metric: metric.into(),
            value,
            unit: unit.into(),
            status: Status::Ok,
            note: String::new(),
        }
    }

    /// Writes a row to the CSV (if any) and prints it.
    pub fn emit(&mut self, row: Row) {
        if !self.args.quiet {
            let v = row.value.map(crate::csv::fmt_value).unwrap_or_default();
            let status = if row.status == Status::Ok { String::new() } else { format!(" [{}]", row.status.as_str()) };
            let note = if row.note.is_empty() { String::new() } else { format!("  ({})", row.note) };
            println!("  {:<34} {:>14} {:<6}{status}{note}", row.metric, v, row.unit);
        }
        if let Some(w) = &mut self.csv
            && let Err(e) = w.write(&row) {
                eprintln!("taskbench: cannot write CSV row: {e}");
            }
        self.rows.push(row);
    }

    /// Emits a numeric metric with parameters.
    pub fn metric(&mut self, params: &str, tasks: Option<u64>, metric: &str, value: f64, unit: &str) {
        let mut r = self.row(metric, Some(value), unit);
        r.params = params.to_string();
        if tasks.is_some() {
            r.tasks = tasks;
        }
        self.emit(r);
    }

    /// Emits a failed or skipped result with a reason.
    pub fn outcome(&mut self, params: &str, tasks: Option<u64>, metric: &str, status: Status, note: &str) {
        let mut r = self.row(metric, None, "");
        r.params = params.to_string();
        r.status = status;
        r.note = note.to_string();
        if tasks.is_some() {
            r.tasks = tasks;
        }
        self.emit(r);
    }

    /// Prints the header line of a benchmark run.
    pub fn banner(&self, extra: &str) {
        if self.args.quiet {
            return;
        }
        let a = &self.args;
        let desc = match &a.imp {
            Impl::Taskbench(c) => c.describe(),
            Impl::Tokio => "tokio multi-thread runtime (stackless reference)".into(),
            Impl::OsThreads => "one OS thread per task".into(),
        };
        println!("taskbench {} | {} | threads={} | {}", a.command, desc, a.threads, extra);
    }

    /// Records the machine specification as text rows.
    pub fn emit_machine_spec(&mut self) {
        let saved = std::mem::replace(&mut self.args.command, "machine-spec".into());
        for (k, v) in crate::machine::collect() {
            let mut r = self.row(&k, None, "");
            r.note = v;
            r.implementation = "machine".into();
            r.strategy = String::new();
            r.reservation_bytes = None;
            r.stack_bytes = None;
            r.threads = None;
            r.tasks = None;
            self.emit(r);
        }
        self.args.command = saved;
    }

    /// Starts this crate's runtime for the selected strategy.
    pub fn runtime(&self) -> Result<Runtime, String> {
        match &self.args.imp {
            Impl::Taskbench(c) => Runtime::with_worker_stack(self.args.threads, c, self.args.worker_stack),
            other => Err(format!("{} does not use the taskbench runtime", other.implementation())),
        }
    }
}

/// Runs benchmark `cmd`. Returns the process exit code: 0 when the benchmark
/// ran (even if a configuration failed, which is a result), 1 when it could not.
pub fn run(args: Args) -> Result<i32, String> {
    crate::machine::raise_nofile_limit();
    let mut ctx = BenchCtx::new(args)?;
    if ctx.args.machine_spec {
        ctx.emit_machine_spec();
    }
    let r = match ctx.args.command.as_str() {
        "spawn-join" => spawn_join::run(&mut ctx),
        "idle-memory" => idle_memory::run(&mut ctx),
        "max-tasks" => max_tasks::run(&mut ctx),
        "echo" => echo::run(&mut ctx, false),
        "idle-connections" => echo::run(&mut ctx, true),
        "deep-recursion" => deep_recursion::run(&mut ctx),
        "starvation" => starvation::run(&mut ctx),
        "probe-overhead" => probe_overhead::run(&mut ctx),
        "cancel-uring" => cancel_uring::run(&mut ctx),
        "machine-spec" => {
            if !ctx.args.machine_spec {
                ctx.emit_machine_spec();
            }
            Ok(())
        }
        other => Err(format!("unknown benchmark {other:?}")),
    };
    r.map(|()| 0)
}

/// `key=value;key=value` from pairs, for the CSV `params` column.
pub fn params(pairs: &[(&str, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";")
}

/// Emits the order statistics of `samples` (nanoseconds) as `<prefix>_p50` etc.
pub fn emit_summary(ctx: &mut BenchCtx, params: &str, tasks: Option<u64>, prefix: &str, samples: &mut [u64]) -> crate::stats::Summary {
    let s = crate::stats::summarize(samples);
    for (name, v) in [
        ("p50", s.p50 as f64),
        ("p90", s.p90 as f64),
        ("p99", s.p99 as f64),
        ("p999", s.p999 as f64),
        ("max", s.max as f64),
        ("mean", s.mean.round()),
    ] {
        ctx.metric(params, tasks, &format!("{prefix}_{name}"), v, "ns");
    }
    ctx.metric(params, tasks, &format!("{prefix}_samples"), s.n as f64, "count");
    s
}
