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
    /// The top of the frame, above its stack slots: the saved registers
    /// and the spill slots, which the entry may write before its probe.
    pub top: u32,
    pub debug: Compiled,
}

/// The bytes of a function's stack slots as Cranelift lays them out: each
/// at an offset aligned to a word or its own alignment, the whole rounded
/// to a word. They are the bottom of the frame (above only the arguments
/// of its calls), which nothing writes before the entry's probe.
pub(crate) fn slots_size(func: &ir::Function) -> u32 {
    let mut end: u32 = 0;
    for (_, data) in func.sized_stack_slots.iter() {
        let align = 8u32.max(1u32 << data.align_shift.min(31));
        end = end.div_ceil(align).saturating_mul(align).saturating_add(data.size);
    }
    end.div_ceil(8).saturating_mul(8)
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

/// Compiles one function in `ctx`, as `Module::define_function` would
/// before it places the code. The context is the thread's own, cleared
/// and used again for each function, which keeps what Cranelift allocated
/// for the last one.
pub(crate) fn compile(isa: &dyn TargetIsa, ctx: &mut Context, id: FuncId) -> Result<Code, String> {
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
        top: code.frame_size.saturating_sub(slots_size(&ctx.func)),
        debug: Compiled::of(code, isa),
    })
}

/// `SISU_NATIVE_CLIF=DIR`: each function's Cranelift IR, as it is handed
/// to be compiled, written to `DIR/N.clif` (`N` counting the functions in
/// the order they are defined), to compare what two ways of writing the IR
/// make (the backend from SIR, the translation of LIR, decision 283).
pub(crate) fn dump_clif(func: &ir::Function) {
    static DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let Some(dir) = DIR.get_or_init(|| std::env::var_os("SISU_NATIVE_CLIF").map(std::path::PathBuf::from))
    else {
        return;
    };
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join(format!("{n:05}.clif")), func.display().to_string());
}

/// How many threads compile: as many as the machine runs at once, at most
/// 8; one means the code generator compiles each function itself.
pub(crate) fn count() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get()).min(8)
}

/// The channels between the code generator and the threads: where jobs
/// go and the threads take them, where the results come back, and where
/// the functions they compiled go back, cleared, for the code generator
/// to write the next ones into (their memory, made on its thread, is used
/// again rather than freed on another).
pub(crate) struct Channels {
    pub jobs: Sender<Job>,
    pub queue: Mutex<Receiver<Job>>,
    pub results: Sender<(usize, Result<Code, String>)>,
    pub done: Receiver<(usize, Result<Code, String>)>,
    pub spare: Sender<ir::Function>,
    pub spares: Receiver<ir::Function>,
}

impl Channels {
    pub fn new() -> Channels {
        let (jobs, queue) = channel();
        let (results, done) = channel();
        let (spare, spares) = channel();
        Channels { jobs, queue: Mutex::new(queue), results, done, spare, spares }
    }
}

/// One thread's work: compile jobs until the code generator closes the
/// queue, or stops the work (it failed, and the code will not be used).
pub(crate) fn work(
    isa: &dyn TargetIsa,
    queue: &Mutex<Receiver<Job>>,
    results: &Sender<(usize, Result<Code, String>)>,
    spare: &Sender<ir::Function>,
    stop: &AtomicBool,
) {
    let mut ctx = Context::new();
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let job = match queue.lock() {
            Ok(q) => q.recv(),
            Err(_) => return,
        };
        let Ok(Job { order, id, func }) = job else { return };
        ctx.clear();
        ctx.func = func;
        let code = compile(isa, &mut ctx, id);
        let mut done = std::mem::replace(&mut ctx.func, ir::Function::new());
        done.clear();
        // The code generator may have stopped taking them back.
        let _ = spare.send(done);
        if results.send((order, code)).is_err() {
            return;
        }
    }
}
