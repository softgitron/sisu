//! The stage-0 interpreter: runs SIR directly (proposal/18-next-steps.md,
//! "The interpreter"). It is the reference semantics until a compiled
//! backend exists.
//!
//! The run loop is a trampoline over an explicit frame stack: a Sisu call
//! pushes a frame and never recurses on the host stack (only intrinsics
//! that call back into Sisu, and drop glue running a `Drop`, nest a run
//! loop). References are places: a frame, a local and a path.
//!
//! Values have value semantics: a move takes the value out of its place,
//! a copy clones it, and every `drop` statement of the SIR runs the
//! value's drop glue (the type's `Drop`, then its parts in reverse order).
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
mod exec;
mod format;
mod host;
mod intrinsics;
mod iter;
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
/// do not use it (the run loop is a trampoline); intrinsics that call back
/// into Sisu (`sort_by`, `Iter.map`, a type's `Drop`) and drop glue for
/// deeply nested values do.
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
}

pub struct Interp<'p, 'h> {
    pub(crate) prog: &'p sir::Program,
    pub(crate) host: &'h mut dyn Host,
    pub(crate) fns: HashMap<&'p str, usize>,
    pub(crate) types: HashMap<&'p str, &'p sir::TypeDecl>,
    /// Variant indices by (type, variant) name.
    pub(crate) variants: HashMap<(&'p str, &'p str), u32>,
    pub(crate) frames: Vec<Frame<'p>>,
    pub(crate) serial: u64,
    /// Whether any type has a `Drop`; without one, drop glue has nothing
    /// observable to do.
    pub(crate) any_drop: bool,
    pub(crate) fuel: Option<u64>,
    pub(crate) fuel_limit: u64,
}

impl<'p, 'h> Interp<'p, 'h> {
    pub fn new(prog: &'p sir::Program, host: &'h mut dyn Host, options: &Options) -> Interp<'p, 'h> {
        let fns = prog.functions.iter().enumerate().map(|(i, f)| (f.name.as_str(), i)).collect();
        let mut types = HashMap::new();
        let mut variants = HashMap::new();
        variants.insert(("Option", "Some"), 0);
        variants.insert(("Option", "None"), 1);
        variants.insert(("Result", "Ok"), 0);
        variants.insert(("Result", "Err"), 1);
        let mut any_drop = false;
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
            variants,
            frames: Vec::new(),
            serial: 0,
            any_drop,
            fuel: options.fuel,
            fuel_limit: options.fuel.unwrap_or(0),
        }
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
    on_big_stack(host, |host| Interp::new(prog, host, options).run_main()).unwrap_or(exit::ABORT)
}

/// Runs every test of the program, in order, each with a fresh
/// interpreter. Returns each test's name and result.
pub fn run_tests(prog: &sir::Program, host: &mut dyn Host, options: &Options) -> Vec<(String, TestResult)> {
    let tests = prog.tests.clone();
    on_big_stack(host, |host| {
        tests.iter().map(|(name, f)| (name.clone(), Interp::new(prog, host, options).run_test(f))).collect()
    })
    .unwrap_or_default()
}

fn on_big_stack<R: Send>(host: &mut dyn Host, f: impl FnOnce(&mut dyn Host) -> R + Send) -> Option<R> {
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
