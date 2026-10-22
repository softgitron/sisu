//! The stage-0 interpreter: runs SIR directly (proposal/18-next-steps.md,
//! "The interpreter"). It is the reference semantics until a compiled
//! backend exists.
//!
//! The run loop is a trampoline over an explicit frame stack: a Sisu call
//! pushes a frame and never recurses on the host stack. References are
//! places: a frame, a local and a path.
//!
//! Values have value semantics: a move takes the value out of its place,
//! a copy clones it, and every `drop` statement of the SIR runs the
//! value's drop glue (a SIR function for a declared type: its `Drop`, then
//! its fields in reverse order) on the same loop (`drops.rs`).
//! A panic unwinds: each block names the cleanup block that runs when
//! anything in it panics, so every value on the stack is dropped in
//! reverse order before the panic reaches `main`. A panic while unwinding
//! aborts (05, "Panics"). Errors are ordinary values; `?` was lowered to a
//! switch and a return that appends to the error's trace.
//!
//! The host intrinsics behind the prelude's `@intrinsic` functions are in
//! `intrinsics.rs`; their contracts are the `##` comments in
//! `compiler/std/*.sisu`. Everything the program does to the world goes
//! through [`Host`], so a run is deterministic under a deterministic host.

mod caps;
pub mod comptime;
pub mod diff;
mod dispatch;
mod drops;
mod exec;
mod float32;
mod format;
pub mod fx;
mod host;
mod intrinsics;
mod iter;
mod live;
mod monitor;
mod ops;
pub mod parallel;
mod place;
pub mod record;
mod sched;
mod sized;
pub mod value;

pub use host::{Host, IoErrorKind, NetFail, Poll, StdHost, TestHost};
pub use live::Rebuild;
pub use sched::{Choice, PCT_STEPS, Stats, Strategy, next_schedule};

use sisu_diag::{Catalog, MsgId, msg};
use sisu_sir as sir;
use std::collections::HashMap;
use value::Value;

/// Calls nested deeper than this panic with a stack overflow. Sisu frames
/// live on the interpreter's own frame stack, not the host's.
pub const MAX_DEPTH: usize = 100_000;

/// The frames beyond [`MAX_DEPTH`] that the unwinding of a panic may
/// push: the drops it runs call code of their own (a `Drop`), which must
/// not overflow again where the stack overflowed (decision 234). A native
/// stack keeps a floor below its probe limit for the same reason.
pub const UNWIND_DEPTH: usize = 1_000;

/// The interpreter runs on a thread with this much host stack. Sisu calls
/// and drops do not use it (the run loop is a trampoline); the host's
/// structural operations on deeply nested values (printing, comparing,
/// hashing, `.copy()`) and `cli.parse` calling a default do.
pub const STACK_BYTES: usize = 256 << 20;

/// Exit statuses of a run.
pub mod exit {
    pub const OK: i32 = 0;
    /// `main` returned an error.
    pub const ERROR: i32 = 1;
    /// A panic reached `main`.
    pub const PANIC: i32 = 101;
    /// A panic while unwinding, or a run that exceeded its step limit.
    pub const ABORT: i32 = 134;
}

/// How a test ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestResult {
    Passed,
    /// The panic message, with its stack.
    Failed(String),
    /// The test reached `todo(name)` (17, D-061).
    Blocked(String),
}

/// Options for a run.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Stop after this many statements (the conformance runner sets it so a
    /// case that loops forever fails instead of hanging).
    pub fuel: Option<u64>,
    /// Run under the dynamic ownership monitor.
    pub monitor: bool,
    /// Count how often each statement runs (the counterexample mode).
    pub coverage: bool,
    /// The seed of the scheduler's choices (`--schedule-seed`).
    pub seed: u64,
    /// Keep the numbers of `--stats`, and which: the scheduler's summary,
    /// the frame depths of the run, or each `Shared` construction site.
    pub stats: bool,
    pub stats_kind: StatsKind,
    /// The seed of a property's random cases (`check`), per test.
    pub case_seed: u64,
    /// The choices of the case a replay token names: `check` runs it first.
    pub replay_case: Option<Vec<i64>>,
    /// How the scheduler chooses (`--scheduler`).
    pub strategy: sched::Strategy,
    /// The `sim` profile (`sisu test --sim`): a test that declares a
    /// `System` gets a simulated one, and capability calls fail with an
    /// error of their declared set at this rate, in parts per million.
    pub sim: Option<u32>,
    /// How many threads run the tests that take no `System` (`--jobs`);
    /// 0 is the default, [`parallel::DEFAULT_JOBS`]. A run reads the same
    /// at any number.
    pub jobs: usize,
    /// The stored snapshots (17, "Snapshots and transcripts"), by
    /// [`snapshot_key`], and whether a changed or missing one is taken
    /// as the new one (`sisu test --update`).
    pub snapshots: std::sync::Arc<std::collections::BTreeMap<String, String>>,
    pub update: bool,
}

/// What `--stats` reports (18, Phases 5 and 6; 20, I-9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatsKind {
    /// Tasks, scheduling latency (Q-34) and `Shared` cells by tag.
    #[default]
    Summary,
    /// `--stats stack-depth`: a histogram of the frame depth at every call
    /// over the run, the deepest stack, and an estimate of the native stack
    /// it would take (16 bytes per SIR local and 64 per frame, 20's
    /// measure of stack exhaustion).
    StackDepth,
    /// `--stats shared`: each `Shared` construction site of the program,
    /// with the escape analysis's tag and the cells it made and that
    /// another task reached (Q-2; the monitor checks the tags).
    Shared,
}

/// The key of a test's `k`-th snapshot (from 1): the test's name, then
/// ` #k` from the second on.
pub fn snapshot_key(test: &str, k: usize) -> String {
    if k <= 1 { test.to_string() } else { format!("{test} #{k}") }
}

/// How a monitored run went, beyond its exit status.
#[derive(Clone, Debug, Default)]
pub struct RunReport {
    pub status: i32,
    /// The monitor stopped the run at an ownership violation.
    pub violation: bool,
    /// How often each source position ran a statement: (function, line,
    /// column).
    pub coverage: HashMap<(String, u32, u32), u32>,
    /// The `--stats` report, when asked for.
    pub stats: Option<String>,
    /// The choices of the failing case a property recorded, for the
    /// test's replay token.
    pub case: Option<Vec<i64>>,
    /// The scheduling choices of a run under a bounded exhaustive search.
    pub trail: Vec<sched::Choice>,
    /// Each snapshot the test took, by [`snapshot_key`], with what it
    /// rendered.
    pub snapshots: Vec<(String, String)>,
    /// The steps the run took: statements and terminators, of every task.
    pub steps: u64,
}

/// Why execution left a frame other than by returning.
#[derive(Debug)]
pub(crate) enum Unwind {
    Panic(Box<PanicInfo>),
    /// Stops the program without running any more Sisu code.
    Abort(String),
    /// From an intrinsic: the running task must wait; the call is retried
    /// with these arguments when it is woken (`sched.rs`).
    Block(Box<sched::Blocked>),
    /// The running task waits or has finished: the scheduler runs another.
    Switch,
}

#[derive(Debug)]
pub(crate) struct PanicInfo {
    pub message: String,
    /// One line per frame the panic left, innermost first.
    pub stack: Vec<String>,
    /// Set when the panic is a typed hole (`todo`).
    pub hole: Option<String>,
    /// Set when the panic is a refinement's predicate failing on a
    /// constant: the type, the value and the predicate, which compile-time
    /// evaluation reports as a diagnostic of its own.
    pub refinement: Option<(String, String, String)>,
    /// Set when the panic is a property's case running past its budget of
    /// steps (`test.catch`): the budget.
    pub too_long: Option<u64>,
    /// Not a panic: a generator dropped before its end unwinds its frames,
    /// running their cleanup, and ends quietly (decision 236).
    pub abandon: bool,
}

/// A catalog message (D-065): run-time messages are catalog entries too.
pub(crate) fn render(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

pub(crate) fn panic(id: MsgId, params: &[(&str, String)]) -> Unwind {
    Unwind::Panic(Box::new(PanicInfo {
        message: render(id, params),
        stack: Vec::new(),
        hole: None,
        refinement: None,
        too_long: None,
        abandon: false,
    }))
}

/// An internal error: the SIR broke an invariant the interpreter relies on.
pub(crate) fn ice(detail: impl std::fmt::Display) -> Unwind {
    Unwind::Abort(render(msg::RUN_INTERNAL, &[("detail", detail.to_string())]))
}

pub(crate) struct Frame<'p> {
    pub serial: u64,
    /// `None` for a scratch frame that holds temporaries for an intrinsic.
    pub func: Option<usize>,
    pub locals: Vec<Value>,
    /// The location of the statement or terminator being executed.
    pub loc: Option<sir::Loc>,
    /// The current block and the index of the next statement in it.
    pub bb: usize,
    pub next: usize,
    /// The panic this frame is unwinding from, while it runs cleanup.
    pub pending: Option<Box<PanicInfo>>,
    /// Where the caller wants the result.
    pub ret: Option<&'p sir::Place>,
    /// The function's type arguments, `$0` and on in its SIR (type
    /// arguments are run-time values in stage 0).
    pub tys: Vec<sir::Type>,
    /// Values to drop before the next statement, the next one last.
    pub drops: Vec<drops::PendingDrop>,
    /// The estimated native stack this frame and the frames under it
    /// take (`--stats stack-depth`).
    pub stack_bytes: u64,
    /// For a frame the run time pushed (an `invariant` check), what to do
    /// with its result.
    pub after: Option<drops::After>,
    /// The frame is leaving by a panic and has no cleanup block: once its
    /// pending drops are done, unwinding continues in the caller.
    pub resume_after_drops: bool,
    /// A frame the scheduler pushed: a task's bottom, a scope's end, a
    /// shield (`sched.rs`).
    pub anchor: Option<sched::Anchor>,
    /// How many locals past the function's own hold what the caller gave
    /// its `Iterable[T]` parameters by value: the parameters refer to them,
    /// and they are dropped when the frame returns or unwinds (decision
    /// 235).
    pub lent: u32,
}

pub struct Interp<'p, 'h> {
    pub(crate) prog: &'p sir::Program,
    pub(crate) host: &'h mut dyn Host,
    /// Every function body the run has: the program's, then those each
    /// epoch of hot replacement added (`live.rs`). A frame holds its body
    /// by index here.
    pub(crate) funcs: Vec<&'p sir::Function>,
    /// The function table of each epoch: a function's name (its slot) to
    /// its body in that epoch. Epoch 0 is the program's.
    pub(crate) epochs: Vec<fx::FxMap<&'p str, usize>>,
    /// Hot replacement, under `--watch`.
    pub(crate) live: Option<live::Live<'h>>,
    pub(crate) types: fx::FxMap<&'p str, &'p sir::TypeDecl>,
    /// The drop glue of each declared type that has one, by type name.
    pub(crate) glue: fx::FxMap<&'p str, usize>,
    /// Variant indices by (type, variant) name.
    pub(crate) variants: fx::FxMap<(&'p str, &'p str), u32>,
    /// Which of the intrinsics' dispatchers answers each intrinsic name,
    /// learnt at its first call (`intrinsics.rs`).
    pub(crate) routes: fx::FxMap<String, intrinsics::Route>,
    /// The text of each string and function constant of the program, by
    /// the address of the constant in its SIR (which outlives the run).
    pub(crate) const_texts: fx::FxMap<usize, std::rc::Rc<str>>,
    /// The body each callee a statement names has in an epoch, by the
    /// epoch and the address of the name in the SIR (which outlives the
    /// run): resolved once rather than at every call.
    pub(crate) callees: fx::FxMap<(usize, usize, usize), usize>,
    /// The locals of frames that returned, emptied, for the next frames
    /// to reuse.
    pub(crate) spare_locals: Vec<Vec<Value>>,
    pub(crate) frames: Vec<Frame<'p>>,
    pub(crate) serial: u64,
    /// Whether any type has a `Drop`; without one, drop glue has nothing
    /// observable to do.
    pub(crate) any_drop: bool,
    pub(crate) fuel: Option<u64>,
    pub(crate) fuel_limit: u64,
    pub(crate) monitor: Option<monitor::Monitor>,
    /// The values `Shared` owns, by cell (a cell is never reused).
    pub(crate) heap: Vec<value::HeapCell>,
    /// Tasks, scopes, channels and clocks.
    pub(crate) sched: sched::Sched<'p>,
    /// The seed of a property's random cases, and the case a replay token
    /// names (17).
    pub(crate) case_seed: u64,
    /// The stored snapshots, and whether a run takes new ones (17).
    pub(crate) snapshots: std::sync::Arc<std::collections::BTreeMap<String, String>>,
    pub(crate) update: bool,
    pub(crate) replay_case: Option<Vec<i64>>,
    /// The failing case a property recorded.
    pub(crate) recorded_case: Option<Vec<i64>>,
    /// The name of the test running, and the snapshots it took (17).
    pub(crate) test_name: String,
    pub(crate) snapshots_taken: Vec<(String, String)>,
    /// For each open `test.catch`, innermost last: the step count at which
    /// its call has run out of steps, and the budget.
    pub(crate) case_ends: Vec<(u64, u64)>,
    /// The `sim` profile (`Options::sim`), its fault rate in parts per
    /// million, and the seeded stream that decides each fault.
    pub(crate) sim: Option<u32>,
    pub(crate) fault_rng: sched::Rng,
    /// What `--stats` reports.
    pub(crate) stats_kind: StatsKind,
}

impl<'p, 'h> Interp<'p, 'h> {
    pub fn new(prog: &'p sir::Program, host: &'h mut dyn Host, options: &Options) -> Interp<'p, 'h> {
        // A real clock is read when it is used, never before (compile-time
        // evaluation refuses the clock).
        let host_clock = if host.virtual_time() {
            sched::RtClock { virtual_: true, now: host.now_nanos() }
        } else {
            sched::RtClock { virtual_: false, now: 0 }
        };
        let fns = prog.functions.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
        let glue = prog
            .functions
            .iter()
            .enumerate()
            .filter(|(_, f)| f.kind == sir::FnKind::Glue)
            .filter_map(|(i, f)| Some((f.name.strip_suffix("#drop")?, i)))
            .collect();
        let mut types = fx::FxMap::default();
        let mut variants = fx::FxMap::default();
        variants.insert(("Option", "Some"), 0);
        variants.insert(("Option", "None"), 1);
        variants.insert(("Result", "Ok"), 0);
        variants.insert(("Result", "Err"), 1);
        // Dropping an owner of a `Shared`, a mutex guard or a capability
        // handle is observable too (a count, a lock), as is any value
        // holding one.
        let mut any_drop = prog.functions.iter().any(|f| f.locals.iter().any(|l| mentions_counted(&l.ty)));
        for t in &prog.types {
            types.insert(t.name(), t);
            match t {
                sir::TypeDecl::Enum { name, variants: vs, drop, .. } => {
                    for (i, v) in vs.iter().enumerate() {
                        variants.insert((name.as_str(), v.name.as_str()), i as u32);
                    }
                    any_drop |= drop.is_some();
                }
                sir::TypeDecl::Struct { drop, .. } => any_drop |= drop.is_some(),
            }
        }
        Interp {
            prog,
            host,
            funcs: prog.functions.iter().collect(),
            epochs: vec![fns],
            live: None,
            types,
            glue,
            variants,
            routes: fx::FxMap::default(),
            const_texts: fx::FxMap::default(),
            callees: fx::FxMap::default(),
            spare_locals: Vec::new(),
            frames: Vec::new(),
            serial: 0,
            any_drop,
            fuel: options.fuel,
            fuel_limit: options.fuel.unwrap_or(0),
            monitor: options.monitor.then(|| monitor::Monitor {
                coverage: options.coverage.then(fx::FxMap::default),
                ..Default::default()
            }),
            heap: Vec::new(),
            sched: sched::Sched::new(options.seed, options.stats, host_clock, options.strategy.clone()),
            case_seed: options.case_seed,
            snapshots: options.snapshots.clone(),
            update: options.update,
            replay_case: options.replay_case.clone(),
            recorded_case: None,
            test_name: String::new(),
            snapshots_taken: Vec::new(),
            case_ends: Vec::new(),
            sim: options.sim,
            fault_rng: sched::Rng::new(options.seed ^ 0x5157_FA17),
            stats_kind: options.stats_kind,
        }
    }

    /// What the monitor saw: whether it stopped the run, and the coverage.
    pub fn report(&self, status: i32) -> RunReport {
        let mut r = RunReport {
            status,
            case: self.recorded_case.clone(),
            trail: self.sched.trail.clone(),
            snapshots: self.snapshots_taken.clone(),
            steps: self.sched.steps,
            ..Default::default()
        };
        if self.sched.keep_stats {
            r.stats = Some(self.stats_report());
        }
        if let Some(m) = &self.monitor {
            r.violation = m.violated.get();
            if let Some(c) = &m.coverage {
                for ((f, line, col), n) in c {
                    if let Some(func) = self.func(*f) {
                        r.coverage.insert((func.name.clone(), *line, *col), *n);
                    }
                }
            }
        }
        r
    }

    /// Runs the program's entry function and reports how it ended on the
    /// host's standard error.
    pub fn run_main(&mut self) -> i32 {
        self.run_main_with(None)
    }

    /// Runs `main`, with `sys` as its `System` if it takes one (else the
    /// real one).
    pub fn run_main_with(&mut self, sys: Option<Value>) -> i32 {
        let Some(entry) = self.prog.entry.clone() else {
            let m = render(msg::RUN_NO_ENTRY, &[]);
            self.host.stderr(&format!("{m}\n"));
            return exit::ERROR;
        };
        let Ok(fi) = self.fn_index(entry.function.as_str()) else {
            let m = render(msg::RUN_INTERNAL, &[("detail", format!("no function `{}`", entry.function))]);
            self.host.stderr(&format!("{m}\n"));
            return exit::ABORT;
        };
        let args = match (entry.takes_system, sys) {
            (true, Some(s)) => vec![s],
            (true, None) => vec![self.system()],
            (false, _) => Vec::new(),
        };
        match self.call_fn(fi, args) {
            Ok(v) => {
                if !entry.returns_result {
                    return exit::OK;
                }
                match v {
                    Value::Variant(a) if a.variant == 1 => {
                        let e = a.fields.into_iter().next().unwrap_or(Value::Unit);
                        let mut text = render(msg::RUN_ERROR_RETURNED, &[("error", self.debug_string(&e))]);
                        text.push('\n');
                        for line in self.trace_lines(&e) {
                            text.push_str("  ");
                            text.push_str(&render(msg::RUN_TRACE_LINE, &[("location", line)]));
                            text.push('\n');
                        }
                        self.host.stderr(&text);
                        exit::ERROR
                    }
                    _ => exit::OK,
                }
            }
            Err(Unwind::Panic(p)) => {
                let text = panic_report(&p);
                self.host.stderr(&text);
                exit::PANIC
            }
            // A save under `--watch` that a swap cannot take: the caller
            // starts the program again, and the message is printed.
            Err(Unwind::Abort(_)) if self.restarting() => exit::ABORT,
            Err(Unwind::Abort(m)) => {
                self.host.stderr(&format!("{m}\n"));
                exit::ABORT
            }
            Err(Unwind::Block(_) | Unwind::Switch) => {
                let m = render(msg::RUN_INTERNAL, &[("detail", "a task switch escaped the run loop".into())]);
                self.host.stderr(&format!("{m}\n"));
                exit::ABORT
            }
        }
    }

    /// Runs one test function.
    pub fn run_bench(&mut self, function: &str, iterations: u64) -> TestResult {
        let Ok(fi) = self.fn_index(function) else {
            return TestResult::Failed(format!(
                "internal error in the interpreter: no function `{function}`"
            ));
        };
        let bench = Value::Struct(Value::adt(
            "test.Bench",
            0,
            vec![Value::Int(iterations.min(i64::MAX as u64) as i64)],
        ));
        match self.call_fn(fi, vec![bench]) {
            Ok(_) => TestResult::Passed,
            Err(Unwind::Panic(p)) => TestResult::Failed(panic_report(&p)),
            Err(Unwind::Abort(m)) => TestResult::Failed(m),
            Err(_) => TestResult::Failed("a task switch escaped the run loop".into()),
        }
    }

    pub fn run_test(&mut self, function: &str) -> TestResult {
        let Ok(fi) = self.fn_index(function) else {
            return TestResult::Failed(format!(
                "internal error in the interpreter: no function `{function}`"
            ));
        };
        self.test_name =
            self.prog.tests.iter().find(|(_, f)| f == function).map(|(n, _)| n.clone()).unwrap_or_default();
        // A test that declares a `System` receives the real one, as `main`
        // does (17, "Isolation and parallelism"); the others make the
        // fakes they need.
        let takes_system = self.func(fi).is_some_and(|f| f.param_count == 1);
        // In the `sim` profile it receives a simulated one instead (17).
        let args = if !takes_system {
            Vec::new()
        } else if self.sim.is_some() {
            match self.call_by_name("System.sim", vec![Value::Int(self.case_seed as i64)]) {
                Ok(sys) => vec![sys],
                Err(Unwind::Panic(p)) => return TestResult::Failed(panic_report(&p)),
                Err(Unwind::Abort(m)) => return TestResult::Failed(m),
                Err(_) => return TestResult::Failed("the simulated system could not be made".into()),
            }
        } else {
            vec![self.system()]
        };
        match self.call_fn(fi, args) {
            Ok(_) => TestResult::Passed,
            Err(Unwind::Panic(p)) => match &p.hole {
                Some(h) => TestResult::Blocked(h.clone()),
                None => TestResult::Failed(panic_report(&p)),
            },
            Err(Unwind::Abort(m)) => TestResult::Failed(m),
            Err(Unwind::Block(_) | Unwind::Switch) => TestResult::Failed(render(
                msg::RUN_INTERNAL,
                &[("detail", "a task switch escaped the run loop".into())],
            )),
        }
    }
}

impl<'p> Interp<'p, '_> {
    /// The body at index `i` of the run's function table.
    pub(crate) fn func(&self, i: usize) -> Option<&'p sir::Function> {
        self.funcs.get(i).copied()
    }
}

/// Whether a type names one whose drop does something besides a `Drop`
/// impl: `Shared` and `Weak` count, a guard unlocks.
fn mentions_counted(t: &sir::Type) -> bool {
    use sir::Type;
    match t {
        Type::Named(n, args) => {
            matches!(
                n.as_str(),
                "Shared"
                    | "Weak"
                    | "MutexGuard"
                    | "Mutex"
                    | "Cell"
                    | "Arena"
                    | "Sender"
                    | "Receiver"
                    | "Cap"
                    | "Fs"
                    | "Net"
                    | "Clock"
                    | "Rand"
                    | "Env"
                    | "Proc"
                    | "Stdin"
                    | "System"
            ) || args.iter().any(mentions_counted)
        }
        Type::Tuple(es) | Type::Dyn(_, es) => es.iter().any(mentions_counted),
        Type::Ref(t, _) => mentions_counted(t),
        Type::Fn(ps, r, _) => ps.iter().any(mentions_counted) || mentions_counted(r),
        Type::Param(_) | Type::Assoc(..) => true,
        _ => false,
    }
}

fn panic_report(p: &PanicInfo) -> String {
    let mut text = render(msg::RUN_PANIC_HEADER, &[("message", p.message.clone())]);
    text.push('\n');
    for line in &sisu_diag::trace::shown(&p.stack) {
        text.push_str("  ");
        text.push_str(line);
        text.push('\n');
    }
    text
}

/// Runs `main` on a thread with a large stack. Returns the exit status.
pub fn run_main(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> i32 {
    run_main_report(prog, host, options).status
}

/// Runs `main`, returning the exit status and what the monitor saw.
pub fn run_main_report(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> RunReport {
    on_big_stack(host, |host| {
        let mut i = Interp::new(prog, host, options);
        let status = i.run_main();
        i.report(status)
    })
    .unwrap_or(RunReport { status: exit::ABORT, ..Default::default() })
}

/// Runs `main` with the fake `System` a test makes (`System.fake(0)`:
/// files in memory, no network, the test clock, seeded random numbers),
/// whatever capabilities `main` asks for: a worksheet's run (10, "REPL and
/// scripts": code that can reach a real capability runs only on an
/// explicit run). The program must keep `System.fake`.
pub fn run_main_faked(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> RunReport {
    on_big_stack(host, |host| {
        let mut i = Interp::new(prog, host, options);
        let status = match i.call_by_name("System.fake", vec![Value::Int(0)]) {
            Ok(sys) => i.run_main_with(Some(sys)),
            Err(Unwind::Panic(p)) => {
                let text = panic_report(&p);
                i.host.stderr(&text);
                exit::PANIC
            }
            Err(_) => exit::ABORT,
        };
        i.report(status)
    })
    .unwrap_or(RunReport { status: exit::ABORT, ..Default::default() })
}

/// `sisu run --watch` (`live.rs`): runs `main`, publishing an epoch of
/// hot replacement for every save that checks and swaps, and starting
/// `main` again with the new program after a save that changes what a
/// swap cannot. Returns how the last run ended. The programs live as long
/// as the process: frames of every epoch point into them.
pub fn run_watch(
    prog: sir::Program,
    host: &mut dyn Host,
    options: &Options,
    rebuild: &mut dyn Rebuild,
) -> RunReport {
    on_big_stack(host, |host| {
        let mut prog: &'static sir::Program = Box::leak(Box::new(prog));
        loop {
            let mut i = Interp::new(prog, &mut *host, options);
            i.live = Some(live::Live { rebuild: &mut *rebuild, restart: None });
            let status = i.run_main();
            let report = i.report(status);
            match i.live.take().and_then(|l| l.restart) {
                Some(next) => prog = Box::leak(Box::new(next)),
                None => return report,
            }
        }
    })
    .unwrap_or(RunReport { status: exit::ABORT, ..Default::default() })
}

/// A transcript's runs of `main` (17, "Snapshots and transcripts"), in
/// one interpreter: each with its arguments and standard input, against
/// a fake `System` whose file system (holding `files`), clock and random
/// numbers persist from one run to the next. Returns what each run wrote
/// (standard output and error, in the order written) and its status.
pub fn run_transcript(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    files: &[(String, String)],
    runs: &[(Vec<String>, Vec<String>)],
) -> Vec<(String, i32)> {
    on_big_stack(host, |host| {
        let written = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let mut held = parallel::Merged { inner: host, out: written.clone() };
        let mut i = Interp::new(prog, &mut held, options);
        let texts = |xs: &[String]| Value::List(xs.iter().map(|x| Value::str(x)).collect());
        let pairs: Vec<Value> =
            files.iter().map(|(p, t)| Value::Tuple(vec![Value::str(p), Value::str(t)])).collect();
        let world = match i.call_by_name("transcript_world", vec![Value::List(pairs)]) {
            Ok(w) => w,
            Err(Unwind::Panic(p)) => return vec![(panic_report(&p), exit::PANIC)],
            Err(Unwind::Abort(m)) => return vec![(m, exit::ABORT)],
            Err(_) => return vec![("the transcript's world could not be made".to_string(), exit::ABORT)],
        };
        let mut out = Vec::new();
        for (args, stdin) in runs {
            // Each run's `System` holds a copy of the world's handles,
            // counted (decision 237), so the world outlives the run.
            i.retain_caps(&world, None);
            let status =
                match i.call_by_name("transcript_system", vec![world.clone(), texts(args), texts(stdin)]) {
                    Ok(sys) => i.run_main_with(Some(sys)),
                    Err(_) => exit::ABORT,
                };
            let text = match written.lock() {
                Ok(mut w) => std::mem::take(&mut *w),
                Err(p) => std::mem::take(&mut *p.into_inner()),
            };
            out.push((text, status));
        }
        out
    })
    .unwrap_or_default()
}

/// Runs every test of the program, in order, each with a fresh
/// interpreter. Returns each test's name and result.
pub fn run_tests(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> Vec<(String, TestResult)> {
    run_tests_report(prog, host, options).into_iter().map(|(n, r, _)| (n, r)).collect()
}

/// Runs the benchmark `function` (`bench#N`) once, with a `Bench` whose
/// loop runs `iterations` times (17, "Benchmarks").
pub fn run_bench(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    function: &str,
    iterations: u64,
) -> TestResult {
    on_big_stack(host, |host| {
        let mut i = Interp::new(prog, host, options);
        i.run_bench(function, iterations)
    })
    .unwrap_or_else(|| TestResult::Failed("the benchmark's thread could not start".into()))
}

/// Runs every test, returning with each result what the monitor saw.
pub fn run_tests_report(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
) -> Vec<(String, TestResult, RunReport)> {
    let all: Vec<usize> = (0..prog.tests.len()).collect();
    run_tests_report_at(prog, host, options, &all)
}

/// Runs the tests at these indices (in declaration order), each with a
/// fresh interpreter, those that take no `System` in parallel
/// (`parallel.rs`).
pub fn run_tests_report_at(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
    which: &[usize],
) -> Vec<(String, TestResult, RunReport)> {
    parallel::run(prog, host, options, which)
}

pub(crate) fn on_big_stack<R: Send>(
    host: &mut dyn Host,
    f: impl FnOnce(&mut dyn Host) -> R + Send,
) -> Option<R> {
    std::thread::scope(|s| {
        let h = std::thread::Builder::new()
            .name("sisu-interp".into())
            .stack_size(STACK_BYTES)
            .spawn_scoped(s, move || f(host));
        match h {
            Ok(h) => h.join().ok(),
            Err(_) => None,
        }
    })
}

#[cfg(test)]
mod tests;
