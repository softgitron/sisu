//! Command-line parsing for the `taskbench` binary.
//!
//! `taskbench <command> [--option value | --option=value | --flag]...`. Every
//! command validates its option names, so a typo is an error rather than a
//! silently ignored setting.

use std::path::PathBuf;

use crate::bench::{self, Args, Impl};
use crate::stack::{StrategyConfig, StrategyId};
use crate::units;

const COMMON: &[&str] = &[
    "strategy", "tasks", "threads", "csv", "machine-spec", "machine-id", "reservation", "stack-size",
    "probe-reserve", "guard", "chunk", "base", "stack-cache", "canary", "worker-stack", "quiet",
];

const FLAGS: &[&str] = &["machine-spec", "canary", "quiet", "break-contract", "scope-per-round", "help"];

fn command_options(cmd: &str) -> Option<&'static [&'static str]> {
    Some(match cmd {
        "spawn-join" => &["rounds", "warmup", "batch", "scope-per-round"],
        "idle-memory" => &["excursion", "settle-ms"],
        "max-tasks" => &["mem-guard", "os-stack"],
        "echo" => &["connections", "msg-bytes", "duration", "warmup-secs"],
        "idle-connections" => &["connections", "settle-ms"],
        "deep-recursion" => &["parse-depth", "naive-depth"],
        "starvation" => &["yield-every", "duration", "probe-interval-us", "baseline-secs"],
        "probe-overhead" => &["reps", "workload", "fib-n", "calls", "tree-depth"],
        "cancel-uring" => &["concurrency", "async-percent", "break-contract", "buf-bytes"],
        "machine-spec" => &[],
        _ => return None,
    })
}

const HELP: &str = "\
taskbench: the bet-3 stack harness (stackful tasks on stacks that cannot move)

USAGE: taskbench <command> [options]

Commands:
  spawn-join        median and p99 of spawn-plus-join        (--rounds 20000 --warmup 2000 --batch 1)
  idle-memory       resident memory per parked task          (--tasks 10000 --excursion 32K --settle-ms 200)
  max-tasks         tasks creatable before a limit stops it  (--tasks CAP=100000 --mem-guard 1G)
  echo              TCP echo, N connections, in-process client (--connections 1000 --msg-bytes 64
                    --duration 3 --warmup-secs 1)
  idle-connections  resident memory per idle connection      (--connections 5000)
  deep-recursion    10k-deep parse and 1M-deep recursion in a task (--parse-depth 10000 --naive-depth 1000000)
  starvation        latency probe under N compute-bound tasks (--tasks 2*threads --yield-every 10000
                    --duration 2 --probe-interval-us 1000 --baseline-secs 1)
  probe-overhead    compute loops with and without stack probes (--reps 7 --workload all|fib|calls|tree)
  cancel-uring      io_uring reads into stack buffers, cancelled in flight (--tasks 4096 --concurrency 512
                    --async-percent 25 [--break-contract])
  machine-spec      print (and with --csv record) the machine specification
  strategies        list the stack strategies
  help              this text

Common options:
  --strategy A|B|C|D|E|tokio|os-threads   (default B)
        A  mmap per task + PROT_NONE guard page, lazy commit
        B  pooled slots from large reservations, software probes, no guard
        C  exact small stacks (2-16 KiB) + probes
        D  A or B (see --base) + MADV_DONTNEED of dead stack pages on park
        E  stackless futures on the same scheduler
        tokio       tokio multi-thread runtime (stackless reference control)
        os-threads  one OS thread per task (spawn-join, idle-memory, max-tasks)
  --threads N            worker threads (default: available CPUs)
  --tasks N              scale (meaning per command, see above)
  --csv PATH             append result rows (schema in README.md)
  --machine-spec         also record uname, CPU, memory, sysctls and limits
  --machine-id ID        machine column (default: host name)
  --reservation SIZE     A/B/D stack reservation per task (default 1M; also 128K, 8M)
  --stack-size SIZE      C exact stack size (default 8K)
  --probe-reserve SIZE   B/C bytes below the probe limit (default B 16K, C 4K)
  --guard SIZE           A guard size (default 4K)
  --chunk SIZE           B/C reservation chunk (default 1G)
  --base A|B             D's base strategy (default A)
  --stack-cache N        A: keep N freed stacks mapped for reuse (default 0)
  --canary               B/C: check a canary at the bottom of each slot on release
  --worker-stack SIZE    worker OS thread stack (default 8M; bounds E's poll recursion)
  --quiet                no per-metric output
";

/// Entry point used by `main`; returns the exit code.
pub fn main() -> i32 {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(argv) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("taskbench: {e}");
            eprintln!("run `taskbench help` for usage");
            2
        }
    }
}

/// Parses and runs a command line (without the program name).
pub fn run(argv: Vec<String>) -> Result<i32, String> {
    let Some(cmd) = argv.first().cloned() else {
        print!("{HELP}");
        return Ok(2);
    };
    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            return Ok(0);
        }
        "strategies" => {
            for id in [StrategyId::A, StrategyId::B, StrategyId::C, StrategyId::D, StrategyId::E] {
                println!("{}", StrategyConfig::new(id).describe());
            }
            println!("tokio: tokio multi-thread runtime (stackless reference control)");
            println!("os-threads: one OS thread per task");
            return Ok(0);
        }
        _ => {}
    }
    let args = parse(&cmd, &argv[1..])?;
    bench::run(args)
}

/// Parses the options of `cmd`.
pub fn parse(cmd: &str, rest: &[String]) -> Result<Args, String> {
    let specific = command_options(cmd).ok_or_else(|| format!("unknown command {cmd:?}"))?;
    let mut args = Args::new(cmd);
    let mut strategy = "B".to_string();
    let mut sc = StrategyConfig::new(StrategyId::B);
    let mut base: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        let raw = &rest[i];
        let Some(body) = raw.strip_prefix("--") else {
            return Err(format!("unexpected argument {raw:?}"));
        };
        let (key, inline) = match body.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (body.to_string(), None),
        };
        if !COMMON.contains(&key.as_str()) && !specific.contains(&key.as_str()) {
            let mut allowed: Vec<&str> = COMMON.iter().chain(specific.iter()).copied().collect();
            allowed.sort_unstable();
            return Err(format!("unknown option --{key} for {cmd}; allowed: --{}", allowed.join(", --")));
        }
        let value = if FLAGS.contains(&key.as_str()) {
            inline.unwrap_or_else(|| "true".into())
        } else if let Some(v) = inline {
            v
        } else {
            i += 1;
            rest.get(i).cloned().ok_or_else(|| format!("--{key} needs a value"))?
        };
        match key.as_str() {
            "strategy" => strategy = value,
            "tasks" => args.tasks = Some(units::parse_count(&value)?),
            "threads" => {
                args.threads = value.parse().map_err(|_| format!("bad --threads {value:?}"))?;
                if args.threads == 0 {
                    return Err("--threads must be at least 1".into());
                }
            }
            "csv" => args.csv = Some(PathBuf::from(value)),
            "machine-spec" => args.machine_spec = value != "false",
            "machine-id" => args.machine_id = value,
            "reservation" => sc.reservation = units::parse_size(&value)?,
            "stack-size" => sc.exact = units::parse_size(&value)?,
            "probe-reserve" => sc.probe_reserve = Some(units::parse_size(&value)?),
            "guard" => sc.guard = units::parse_size(&value)?,
            "chunk" => sc.chunk = units::parse_size(&value)?,
            "base" => base = Some(value),
            "stack-cache" => sc.cache = value.parse().map_err(|_| format!("bad --stack-cache {value:?}"))?,
            "canary" => sc.canary = value != "false",
            "worker-stack" => args.worker_stack = units::parse_size(&value)?,
            "quiet" => args.quiet = value != "false",
            _ => {
                args.opts.insert(key, value);
            }
        }
        i += 1;
    }
    args.imp = match strategy.to_ascii_lowercase().as_str() {
        "tokio" => Impl::Tokio,
        "os-threads" | "threads" => Impl::OsThreads,
        s => {
            let id = StrategyId::parse(s).ok_or_else(|| format!("unknown strategy {strategy:?}"))?;
            sc.id = id;
            if let Some(b) = &base {
                sc.decommit_base = StrategyId::parse(b).ok_or_else(|| format!("unknown --base {b:?}"))?;
            }
            sc.validate()?;
            Impl::Taskbench(sc)
        }
    };
    if base.is_some() && !matches!(&args.imp, Impl::Taskbench(c) if c.id == StrategyId::D) {
        return Err("--base applies to strategy D only".into());
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_common_and_specific_options() {
        let a = parse("spawn-join", &v("--strategy A --reservation 128K --threads 2 --rounds=500 --csv out.csv --machine-spec")).unwrap();
        let Impl::Taskbench(c) = &a.imp else { panic!() };
        assert_eq!(c.id, StrategyId::A);
        assert_eq!(c.reservation, 128 * 1024);
        assert_eq!(a.threads, 2);
        assert_eq!(a.count("rounds", 1).unwrap(), 500);
        assert!(a.machine_spec);
        assert_eq!(a.csv.as_deref(), Some(std::path::Path::new("out.csv")));
    }

    #[test]
    fn strategy_d_with_base() {
        let a = parse("idle-memory", &v("--strategy D --base B --tasks 100k")).unwrap();
        let Impl::Taskbench(c) = &a.imp else { panic!() };
        assert_eq!(c.label(), "D(B)");
        assert_eq!(a.tasks, Some(100_000));
        assert!(parse("idle-memory", &v("--strategy A --base B")).is_err());
    }

    #[test]
    fn rejects_unknown_options_and_values() {
        assert!(parse("spawn-join", &v("--roundz 5")).unwrap_err().contains("unknown option"));
        assert!(parse("echo", &v("--rounds 5")).is_err(), "rounds is not an echo option");
        assert!(parse("spawn-join", &v("--strategy Z")).is_err());
        assert!(parse("spawn-join", &v("--threads")).is_err());
        assert!(parse("nonsense", &[]).is_err());
        assert!(parse("deep-recursion", &v("--strategy C --stack-size 1K")).is_err(), "C below 2K");
    }

    #[test]
    fn controls_and_flags() {
        assert_eq!(parse("echo", &v("--strategy tokio")).unwrap().imp, Impl::Tokio);
        assert_eq!(parse("max-tasks", &v("--strategy os-threads")).unwrap().imp, Impl::OsThreads);
        let a = parse("cancel-uring", &v("--break-contract --strategy C")).unwrap();
        assert!(a.flag("break-contract"));
    }
}
