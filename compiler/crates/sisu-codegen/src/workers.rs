//! The threads that compile (decision 219): the code generator writes each
//! function's Cranelift IR on the thread that called it and hands it here;
//! each thread compiles what it takes to machine code, and the code
//! generator defines the results in the object file in the order it made
//! the functions, so the object is the same however the threads ran.

use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use cranelift_codegen::{CompiledCode, Context, ir};
use cranelift_module::{FuncId, ModuleError, ModuleReloc};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

/// A function to compile: its place in the order the code generator made
/// the functions, its id, and its IR.
pub(crate) struct Job {
    pub order: usize,
    pub id: FuncId,
    pub func: ir::Function,
}

/// A function compiled: its machine code and what the object file needs
/// to place it, or Cranelift's error, as the module would report it; and
/// what the debug information needs of it (decision 227).
pub(crate) struct Code {
    pub bytes: Vec<u8>,
    pub alignment: u64,
    pub relocs: Vec<ModuleReloc>,
    pub frame_size: u32,
    pub debug: Compiled,
}

/// What the debug information needs of a compiled function: the source
/// location of each run of its code, each stack slot's offset, and how to
/// unwind it.
pub(crate) struct Compiled {
    pub lines: Vec<(u32, u32, u32)>,
    pub slots: Vec<u32>,
    pub unwind: Option<UnwindInfo>,
}

impl Compiled {
    pub(crate) fn of(code: &CompiledCode, isa: &dyn TargetIsa) -> Compiled {
        Compiled {
            lines: code.buffer.get_srclocs_sorted().iter().map(|s| (s.start, s.end, s.loc.bits())).collect(),
            slots: code.sized_stackslot_offsets.values().copied().collect(),
            unwind: code.create_unwind_info(isa).ok().flatten(),
        }
    }
}

/// Compiles one function, as `Module::define_function` would before it
/// places the code.
pub(crate) fn compile(isa: &dyn TargetIsa, id: FuncId, func: ir::Function) -> Result<Code, String> {
    let mut ctx = Context::for_function(func);
    let alignment = match ctx.compile(isa, &mut ControlPlane::default()) {
        Ok(code) => u64::from(code.buffer.alignment),
        Err(e) => return Err(format!("{:?}", ModuleError::from(e))),
    };
    let Some(code) = ctx.compiled_code() else { return Err("no code".to_string()) };
    let relocs =
        code.buffer.relocs().iter().map(|r| ModuleReloc::from_mach_reloc(r, &ctx.func, id)).collect();
    Ok(Code {
        bytes: code.buffer.data().to_vec(),
        alignment,
        relocs,
        frame_size: code.frame_size,
        debug: Compiled::of(code, isa),
    })
}

/// How many threads compile: as many as the machine runs at once, at most
/// 8; one means the code generator compiles each function itself.
pub(crate) fn count() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get()).min(8)
}

/// The channels between the code generator and the threads: where jobs
/// go and the threads take them, and where the results come back.
pub(crate) struct Channels {
    pub jobs: Sender<Job>,
    pub queue: Mutex<Receiver<Job>>,
    pub results: Sender<(usize, Result<Code, String>)>,
    pub done: Receiver<(usize, Result<Code, String>)>,
}

impl Channels {
    pub fn new() -> Channels {
        let (jobs, queue) = channel();
        let (results, done) = channel();
        Channels { jobs, queue: Mutex::new(queue), results, done }
    }
}

/// One thread's work: compile jobs until the code generator closes the
/// queue, or stops the work (it failed, and the code will not be used).
pub(crate) fn work(
    isa: &dyn TargetIsa,
    queue: &Mutex<Receiver<Job>>,
    results: &Sender<(usize, Result<Code, String>)>,
    stop: &AtomicBool,
) {
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let job = match queue.lock() {
            Ok(q) => q.recv(),
            Err(_) => return,
        };
        let Ok(Job { order, id, func }) = job else { return };
        if results.send((order, compile(isa, id, func))).is_err() {
            return;
        }
    }
}
