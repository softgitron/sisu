//! The stage-0 interpreter: runs SIR directly (proposal/18-next-steps.md,
//! "The interpreter"). It is the reference semantics until a compiled
//! backend exists.
//!
//! The run loop is a trampoline over an explicit frame stack: a Sisu call
//! pushes a frame and never recurses on the host stack (only `cli.parse`,
//! calling a field's default, nests a run loop). References are places: a
//! frame, a local and a path.
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

mod cli;
pub mod comptime;
mod dispatch;
mod drops;
mod exec;
mod format;
mod host;
mod intrinsics;
mod iter;
mod monitor;
mod ops;
mod place;
pub mod value;

pub use host::{Host, IoErrorKind, StdHost, TestHost};

use sisu_diag::{Catalog, MsgId, msg};
use sisu_sir as sir;
use std::collections::HashMap;
use std::rc::Rc;
use value::Value;

/// Calls nested deeper than this panic with a stack overflow. Sisu frames
/// live on the interpreter's own frame stack, not the host's.
pub const MAX_DEPTH: usize = 100_000;

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
}

/// Why execution left a frame other than by returning.
#[derive(Debug)]
pub(crate) enum Unwind {
    Panic(Box<PanicInfo>),
    /// Stops the program without running any more Sisu code.
    Abort(String),
}

#[derive(Debug)]
pub(crate) struct PanicInfo {
    pub message: String,
    /// One line per frame the panic left, innermost first.
    pub stack: Vec<String>,
    /// Set when the panic is a typed hole (`todo`).
    pub hole: Option<String>,
}

/// A catalog message (D-065): run-time messages are catalog entries too.
pub(crate) fn render(id: MsgId, params: &[(&str, String)]) -> String {
    Catalog::builtin().render(id.id(), params)
}

pub(crate) fn panic(id: MsgId, params: &[(&str, String)]) -> Unwind {
    Unwind::Panic(Box::new(PanicInfo { message: render(id, params), stack: Vec::new(), hole: None }))
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
    /// For a frame the run time pushed (an `invariant` check), what to do
    /// with its result.
    pub after: Option<drops::After>,
    /// The frame is leaving by a panic and has no cleanup block: once its
    /// pending drops are done, unwinding continues in the caller.
    pub resume_after_drops: bool,
}

pub struct Interp<'p, 'h> {
    pub(crate) prog: &'p sir::Program,
    pub(crate) host: &'h mut dyn Host,
    pub(crate) fns: HashMap<&'p str, usize>,
    pub(crate) types: HashMap<&'p str, &'p sir::TypeDecl>,
    /// The drop glue of each declared type that has one, by type name.
    pub(crate) glue: HashMap<&'p str, usize>,
    /// Variant indices by (type, variant) name.
    pub(crate) variants: HashMap<(&'p str, &'p str), u32>,
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
}

impl<'p, 'h> Interp<'p, 'h> {
    pub fn new(prog: &'p sir::Program, host: &'h mut dyn Host, options: &Options) -> Interp<'p, 'h> {
        let fns = prog.functions.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
        let glue = prog
            .functions
            .iter()
            .enumerate()
            .filter(|(_, f)| f.kind == sir::FnKind::Glue)
            .filter_map(|(i, f)| Some((f.name.strip_suffix("#drop")?, i)))
            .collect();
        let mut types = HashMap::new();
        let mut variants = HashMap::new();
        variants.insert(("Option", "Some"), 0);
        variants.insert(("Option", "None"), 1);
        variants.insert(("Result", "Ok"), 0);
        variants.insert(("Result", "Err"), 1);
        // Dropping an owner of a `Shared` or a mutex guard is observable
        // too (a count, a lock), as is any value holding one.
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
            fns,
            types,
            glue,
            variants,
            frames: Vec::new(),
            serial: 0,
            any_drop,
            fuel: options.fuel,
            fuel_limit: options.fuel.unwrap_or(0),
            monitor: options.monitor.then(|| monitor::Monitor {
                coverage: options.coverage.then(HashMap::new),
                ..Default::default()
            }),
            heap: Vec::new(),
        }
    }

    /// What the monitor saw: whether it stopped the run, and the coverage.
    pub fn report(&self, status: i32) -> RunReport {
        let mut r = RunReport { status, ..Default::default() };
        if let Some(m) = &self.monitor {
            r.violation = m.violated.get();
            if let Some(c) = &m.coverage {
                for ((f, line, col), n) in c {
                    if let Some(func) = self.prog.functions.get(*f) {
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
        let Some(entry) = self.prog.entry.clone() else {
            let m = render(msg::RUN_NO_ENTRY, &[]);
            self.host.stderr(&format!("{m}\n"));
            return exit::ERROR;
        };
        let Some(&fi) = self.fns.get(entry.function.as_str()) else {
            let m = render(msg::RUN_INTERNAL, &[("detail", format!("no function `{}`", entry.function))]);
            self.host.stderr(&format!("{m}\n"));
            return exit::ABORT;
        };
        let args = if entry.takes_system { vec![self.system()] } else { Vec::new() };
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
            Err(Unwind::Abort(m)) => {
                self.host.stderr(&format!("{m}\n"));
                exit::ABORT
            }
        }
    }

    /// Runs one test function.
    pub fn run_test(&mut self, function: &str) -> TestResult {
        let Some(&fi) = self.fns.get(function) else {
            return TestResult::Failed(format!(
                "internal error in the interpreter: no function `{function}`"
            ));
        };
        match self.call_fn(fi, Vec::new()) {
            Ok(_) => TestResult::Passed,
            Err(Unwind::Panic(p)) => match &p.hole {
                Some(h) => TestResult::Blocked(h.clone()),
                None => TestResult::Failed(panic_report(&p)),
            },
            Err(Unwind::Abort(m)) => TestResult::Failed(m),
        }
    }

    fn system(&self) -> Value {
        let fs =
            Value::Fs(Rc::new(value::FsCap { store: value::FsStore::Host, read_only: false, root: None }));
        Value::Struct(Value::adt("System", 0, vec![fs, Value::Env, Value::Clock]))
    }
}

/// Whether a type names one whose drop does something besides a `Drop`
/// impl: `Shared` and `Weak` count, a guard unlocks.
fn mentions_counted(t: &sir::Type) -> bool {
    use sir::Type;
    match t {
        Type::Named(n, args) => {
            matches!(n.as_str(), "Shared" | "Weak" | "MutexGuard" | "Mutex" | "Cell" | "Arena")
                || args.iter().any(mentions_counted)
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
    for line in &p.stack {
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

/// Runs every test of the program, in order, each with a fresh
/// interpreter. Returns each test's name and result.
pub fn run_tests(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> Vec<(String, TestResult)> {
    run_tests_report(prog, host, options).into_iter().map(|(n, r, _)| (n, r)).collect()
}

/// Runs every test, returning with each result what the monitor saw.
pub fn run_tests_report(
    prog: &sir::Program,
    host: &mut dyn Host,
    options: &Options,
) -> Vec<(String, TestResult, RunReport)> {
    let tests = prog.tests.clone();
    on_big_stack(host, |host| {
        tests
            .iter()
            .map(|(name, f)| {
                let mut i = Interp::new(prog, host, options);
                let r = i.run_test(f);
                let status = if matches!(r, TestResult::Passed) { exit::OK } else { exit::ERROR };
                (name.clone(), r, i.report(status))
            })
            .collect()
    })
    .unwrap_or_default()
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
