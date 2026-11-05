//! The native debug backend (proposal/18-next-steps.md, "Bootstrapping",
//! stage 2; 10, "A non-LLVM backend for debug builds"): SIR, exactly as
//! the ownership checker and `sisu sir` leave it, compiled to machine code
//! with Cranelift into an object file, and linked with the system's `cc`
//! against `sisu-rt`, the runtime.
//!
//! Generic functions are compiled once per list of type arguments a call
//! gives them (monomorphised), from the entry; a call through a trait is
//! resolved to its impl's method when the instance is compiled, except on
//! a `dyn`, whose dispatcher chooses by the type of its value. Every value
//! lives in memory, in the layout `sisu_rt::abi` describes, and every
//! function takes the addresses of its result and arguments and returns
//! whether a panic is leaving it, so that unwinding is the cleanup blocks
//! of SIR run as ordinary control flow. docs/architecture.md, "Stage 2",
//! has the mapping of each construct.

mod bodies;
mod debug;
mod facts;
mod func;
mod fx;
mod layout;
mod lir;
mod translate;
mod workers;

pub use debug::Sources;
pub use translate::{compile_lir, lir_is_release};

use cranelift_codegen::ir::{self, AbiParam, InstBuilder, types};
use cranelift_codegen::ir::{MemFlags, StackSlotData, StackSlotKind};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, ir::Signature};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use layout::Types;
use sisu_rt::abi::{self, Intrinsic, sym};
use sisu_sir as sir;
use sisu_sir::{ImplTarget, Type};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Why a program was not compiled or linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A construct this round of the backend does not compile.
    Unsupported { what: String, function: String },
    /// The backend broke one of its own rules.
    Internal(String),
    /// The system linker failed.
    Link(String),
    /// The runtime library is not where the driver looks.
    NoRuntime(String),
}

impl Error {
    pub(crate) fn unsupported(what: impl Into<String>) -> Error {
        Error::Unsupported { what: what.into(), function: String::new() }
    }

    pub(crate) fn internal(what: impl Into<String>) -> Error {
        Error::Internal(what.into())
    }

    /// The error, saying which function it was found in.
    fn in_function(self, f: &str) -> Error {
        match self {
            Error::Unsupported { what, function } if function.is_empty() => {
                Error::Unsupported { what, function: f.to_string() }
            }
            other => other,
        }
    }
}

/// What a call through a trait runs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    Fn(String, Vec<Type>),
    Intrinsic(String, Vec<Type>),
    /// A method of a trait called on a `dyn`, with the `dyn` as `Self`:
    /// the impl is chosen when the program runs, by a dispatcher.
    Dyn(String, Vec<Type>),
}

/// What compiling a program made.
pub struct Compiled {
    /// The object file.
    pub object: Vec<u8>,
    /// Function instances compiled (each generic function once per list of
    /// type arguments).
    pub functions: usize,
    /// SIR statements and terminators compiled.
    pub statements: usize,
    /// The checks the release profile compiled and those its facts left
    /// out (decision 270).
    pub checks: Checks,
}

/// The checks of a build: index and overflow checks compiled, those of
/// them the facts prove unneeded, and the tests the facts decide (in the
/// release profile, decision 270).
#[derive(Clone, Copy, Debug, Default)]
pub struct Checks {
    pub index: usize,
    pub index_proven: usize,
    pub overflow: usize,
    pub overflow_proven: usize,
    pub tests_decided: usize,
}

/// Compiles a program to an object file defining the C `main`, with debug
/// information whose source paths are relative to the current directory.
pub fn compile(prog: &sir::Program) -> Result<Compiled, Error> {
    let comp_dir = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
    compile_for(prog, &Sources { comp_dir, std_dir: None })
}

/// [`compile`], with the debug information saying where the sources are.
pub fn compile_for(prog: &sir::Program, sources: &Sources) -> Result<Compiled, Error> {
    compile_with_options(prog, &Options { sources: sources.clone(), release: false })
}

/// How a program is compiled: where its sources are, for the debug
/// information, and the profile (09, "Build"; decision 232).
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub sources: Sources,
    /// The `release` profile: Cranelift's optimisations (its `speed`
    /// level) rather than none. The checks of overflow and bounds are the
    /// same, as 09 asks; the contracts are lowering's business.
    pub release: bool,
}

/// [`compile`], with the options of the build.
pub fn compile_with_options(prog: &sir::Program, options: &Options) -> Result<Compiled, Error> {
    compile_with(prog, false, options).map(|(c, _)| c)
}

/// The runtime in Sisu (`compiler/rt`, decision 336) compiled to an object:
/// each `export_NAME` function of its root module (a name without a dot,
/// without type parameters) is the global symbol `sisu_rt_NAME` of Sisu's
/// convention, and what it calls is compiled with it under local symbols.
/// Every function is the runtime's own code: it counts no frame, passes no
/// yield point and adds no line to a stack trace, and none may walk a value
/// by the program's type table, which the library has none of (a construct
/// that would is `Unsupported`). There is no C `main` and no type table.
pub fn compile_library(prog: &sir::Program, options: &Options) -> Result<Compiled, Error> {
    let mut cg = Codegen::new(prog, 1, options.release, true)?;
    cg.library = true;
    let mut exports: Vec<(&str, String)> = Vec::new();
    for f in &prog.functions {
        if let Some(rest) = f.name.strip_prefix("export_")
            && !f.name.contains('.')
            && f.type_params.is_empty()
            && f.kind == sir::FnKind::Normal
        {
            exports.push((f.name.as_str(), format!("sisu_rt_{rest}")));
        }
    }
    if exports.is_empty() {
        return Err(Error::unsupported("a runtime library without an `export_` function"));
    }
    for (name, symbol) in &exports {
        cg.export_instance(name, symbol)?;
    }
    cg.complete()?;
    let functions = cg.compiled;
    let statements = cg.statements;
    let checks = cg.checks;
    let mut product = cg.module.finish();
    debug::write(
        &mut product,
        &prog.sources,
        &cg.debug,
        debug::Describe::Sir(&mut cg.types),
        &options.sources,
        &*cg.isa,
    )?;
    let object = product.emit().map_err(|e| Error::internal(format!("writing the object file: {e}")))?;
    Ok(Compiled { object, functions, statements, checks })
}

/// Every construct of the program that the backend does not compile yet,
/// with the function it is in and how many functions have it, rather than
/// the first: how far a program is from compiling (a function is left at
/// its first such construct).
pub fn survey(prog: &sir::Program) -> Result<Vec<(String, String, usize)>, Error> {
    let (_, missing) = compile_with(prog, true, &Options::default())?;
    let mut counted: Vec<(String, String, usize)> = Vec::new();
    for (what, function) in missing {
        match counted.iter_mut().find(|(w, _, _)| *w == what) {
            Some(c) => c.2 += 1,
            None => counted.push((what, function, 1)),
        }
    }
    counted.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    Ok(counted)
}

fn compile_with(
    prog: &sir::Program,
    survey: bool,
    options: &Options,
) -> Result<(Compiled, Vec<(String, String)>), Error> {
    // std.par's `map` runs its bodies on the runtime's workers (decision
    // 333).
    let prog = &*bodies::native_bodies(prog);
    // A function value holds its closure's environment, as many words as
    // the largest `own` closure's (decision 224), which only compiling
    // finds: a program that needs more is compiled again with them.
    let mut env_words = 1;
    loop {
        match compile_once(prog, survey, env_words, options)? {
            Ok(done) => return Ok(done),
            Err(more) => env_words = more,
        }
    }
}

/// Compiles the program with function values of `env_words` words of
/// environment; `Err` with the words it needs when an `own` closure's
/// environment is larger.
#[allow(clippy::type_complexity)]
fn compile_once(
    prog: &sir::Program,
    survey: bool,
    env_words: u32,
    options: &Options,
) -> Result<Result<(Compiled, Vec<(String, String)>), u32>, Error> {
    let Some(entry) = &prog.entry else {
        return Err(Error::unsupported("a program without `main`"));
    };
    let mut tt = std::time::Instant::now();
    let mut cg = Codegen::new(prog, env_words, options.release, false)?;
    cg.survey = survey;
    let threads = workers::count();
    let (main_id, flags, ret_ty) = if threads > 1 {
        let workers::Channels { jobs, queue, results, done, spare, spares } = workers::Channels::new();
        // A thread fewer than the machine runs while this one writes the
        // IR; then it compiles too.
        let senders: Vec<_> = (1..threads).map(|_| results.clone()).collect();
        let isa = cg.isa.clone();
        let stop = std::sync::atomic::AtomicBool::new(false);
        cg.jobs = Some(jobs);
        cg.spares = Some(spares);
        std::thread::scope(|s| {
            // A thread that cannot be made leaves its share to the others,
            // and to this one.
            let handles: Vec<_> = senders
                .into_iter()
                .filter_map(|r| {
                    let (isa, queue, stop, spare) = (&isa, &queue, &stop, spare.clone());
                    std::thread::Builder::new()
                        .spawn_scoped(s, move || workers::work(&**isa, queue, &r, &spare, stop))
                        .ok()
                })
                .collect();
            let generated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cg.generate(entry)));
            step("the IR written", &mut tt);
            // Closing the queue lets the threads finish what is in it; a
            // failure stops them, since nothing they compile will be used.
            cg.jobs = None;
            cg.spares = None;
            if matches!(generated, Ok(Ok(_))) {
                workers::work(&*isa, &queue, &results, &spare, &stop);
            } else {
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            drop(results);
            for h in handles {
                if let Err(e) = h.join() {
                    std::panic::resume_unwind(e);
                }
            }
            step("the rest compiled", &mut tt);
            let main = generated.unwrap_or_else(|e| std::panic::resume_unwind(e))?;
            cg.place(done.try_iter().collect())?;
            Ok::<_, Error>(main)
        })?
    } else {
        let main = cg.generate(entry)?;
        step("the IR written and compiled", &mut tt);
        main
    };
    if cg.env_needed > env_words {
        return Ok(Err(cg.env_needed));
    }
    cg.define_main(main_id, flags, ret_ty)?;
    let functions = cg.compiled;
    let statements = cg.statements;
    let checks = cg.checks;
    let missing = std::mem::take(&mut cg.missing);
    let mut product = cg.module.finish();
    step("the code placed", &mut tt);
    debug::write(
        &mut product,
        &prog.sources,
        &cg.debug,
        debug::Describe::Sir(&mut cg.types),
        &options.sources,
        &*cg.isa,
    )?;
    step("the debug information written", &mut tt);
    let object = product.emit().map_err(|e| Error::internal(format!("writing the object file: {e}")))?;
    step("the object emitted", &mut tt);
    Ok(Ok((Compiled { object, functions, statements, checks }, missing)))
}

/// `SISU_NATIVE_TIMES=1`, as the driver reads it: a line per step of code
/// generation saying how long it took, for measuring the backend.
fn step(name: &str, since: &mut std::time::Instant) {
    static TIMES: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *TIMES.get_or_init(|| std::env::var_os("SISU_NATIVE_TIMES").is_some()) {
        eprintln!("time: {name} {:.1} ms", since.elapsed().as_secs_f64() * 1e3);
    }
    *since = std::time::Instant::now();
}

/// Whether a type is concrete as it is: it has no type parameter, no
/// associated type and no constant operation to put arguments in or look
/// up, so that [`Codegen::concrete`] gives it unchanged.
fn settled(t: &Type) -> bool {
    match t {
        Type::Param(_) | Type::Assoc(..) | Type::ConstOp(..) => false,
        Type::Named(_, a) | Type::Tuple(a) | Type::Dyn(_, a) => a.iter().all(settled),
        Type::Fn(p, r, _) => p.iter().all(settled) && settled(r),
        Type::Ref(x, _) => settled(x),
        _ => true,
    }
}

/// The system libraries a Rust static library needs on this target.
fn native_libs() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["-lSystem", "-lc", "-lm"]
    } else {
        &["-lgcc_s", "-lutil", "-lrt", "-lpthread", "-lm", "-ldl", "-lc"]
    }
}

/// The directories searched for programs: the search path, else the usual
/// ones (the conformance runner gives programs an empty environment).
fn search_path() -> std::ffi::OsString {
    std::env::var_os("PATH")
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/local/bin:/usr/bin:/bin"))
}

/// The program `name` on the search path.
fn find_program(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&search_path()).map(|d| d.join(name)).find(|p| p.is_file())
}

/// Links an object file with the runtime library into an executable.
pub fn link(object: &[u8], runtime: &Path, rt: &Path, output: &Path) -> Result<(), Error> {
    let obj = output.with_extension("o");
    std::fs::write(&obj, object)
        .map_err(|e| Error::Link(format!("cannot write `{}`: {e}", obj.display())))?;
    // `cc` finds `ld` and its helpers on the search path, which an empty
    // environment (the conformance runner's) does not have. LLVM's `lld`,
    // where there is one, links a quarter of the time the system's `ld`
    // takes.
    let mut cmd = std::process::Command::new(find_program("cc").unwrap_or_else(|| PathBuf::from("cc")));
    cmd.env("PATH", search_path());
    if find_program("ld.lld").is_some() {
        cmd.arg("-fuse-ld=lld");
    }
    // The sections nothing reaches from `main` are left out (decision
    // 682): the runtime in Rust's functions the program does not call, and
    // the runtime in Sisu's, one section a function.
    cmd.arg(if cfg!(target_os = "macos") { "-Wl,-dead_strip" } else { "-Wl,--gc-sections" });
    // The runtime in Sisu's object, then the runtime in Rust (decision 336).
    let out = cmd.arg("-o").arg(output).arg(&obj).arg(rt).arg(runtime).args(native_libs()).output();
    // `SISU_NATIVE_KEEP=1` keeps the object file beside the executable.
    if std::env::var_os("SISU_NATIVE_KEEP").is_none() {
        let _ = std::fs::remove_file(&obj);
    }
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(Error::Link(String::from_utf8_lossy(&o.stderr).trim().to_string())),
        Err(e) => Err(Error::Link(format!("cannot run `cc`: {e}"))),
    }
}

/// The runtime library cargo built beside the `sisu` executable `exe`:
/// `libsisu_rt.a` there, or the newest `deps/libsisu_rt-*.a` (which a
/// `cargo test` builds without copying it up), whichever is newer.
pub fn find_runtime(exe: &Path) -> Result<PathBuf, Error> {
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut found: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut consider = |p: PathBuf| {
        if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified())
            && found.as_ref().is_none_or(|(t, _)| m > *t)
        {
            found = Some((m, p));
        }
    };
    consider(dir.join("libsisu_rt.a"));
    if let Ok(entries) = std::fs::read_dir(dir.join("deps")) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("libsisu_rt-") && name.ends_with(".a") {
                consider(e.path());
            }
        }
    }
    found.map(|(_, p)| p).ok_or_else(|| Error::NoRuntime(dir.display().to_string()))
}

/// What the code generator keeps across the functions of a program.
pub(crate) struct Codegen<'p> {
    pub prog: &'p sir::Program,
    /// The program's functions by name, the first of each name, as
    /// [`sir::Program::function`] finds them by searching.
    fns: fx::FxMap<&'p str, &'p sir::Function>,
    pub module: ObjectModule,
    pub types: Types<'p>,
    pub call_conv: CallConv,
    /// Every instance declared, by function and type arguments.
    instances: fx::FxMap<(String, Vec<Type>), FuncId>,
    queue: VecDeque<(String, Vec<Type>, FuncId)>,
    /// The runtime's entry points, declared once.
    runtime: fx::FxMap<&'static str, FuncId>,
    /// The foreign functions of `extern` blocks a call names, with their C
    /// signatures (decision 302).
    c_functions: fx::FxMap<String, (FuncId, Vec<ir::Type>, Vec<ir::Type>)>,
    pub depth: DataId,
    /// The scheduler's budget, the running task's stack limits, and the
    /// site of the last call that may wait (`sisu_rt::abi::sym`).
    pub budget: DataId,
    pub stack: DataId,
    pub site: DataId,
    /// The largest top of a probed function's frame, and the largest frame
    /// of a function without a probe, in bytes: the probe reserve of a
    /// task's stack holds them (decisions 203 and 234).
    max_top: u32,
    max_unprobed: u32,
    /// Texts and call descriptors in the read-only data, by content.
    texts: fx::FxMap<Vec<u8>, DataId>,
    descs: fx::FxMap<Vec<u32>, DataId>,
    /// The glue instances the type table names, as they are declared.
    glue_ids: Vec<FuncId>,
    /// The capability handles of which the program makes objects other
    /// than the host's (fakes, attenuations, test clocks): a call through
    /// one of these must look at the object, which this round of the
    /// backend does not.
    pub foreign: std::collections::HashSet<String>,
    /// The function each named function has as a value: it takes an
    /// environment it ignores, so that every function value is called the
    /// way a closure is.
    thunks: fx::FxMap<String, (FuncId, Type)>,
    fbctx: FunctionBuilderContext,
    /// The release profile (decision 232): besides Cranelift's
    /// optimisations, scalars in registers (decision 246).
    pub release: bool,
    /// The target, which the threads that compile share (decision 219).
    isa: cranelift_codegen::isa::OwnedTargetIsa,
    /// Where the functions go to be compiled on those threads; none when
    /// each is compiled as it is made.
    jobs: Option<std::sync::mpsc::Sender<workers::Job>>,
    /// The functions the threads compiled, cleared, to write the next
    /// ones into.
    spares: Option<std::sync::mpsc::Receiver<ir::Function>>,
    /// The functions handed to the threads, in the order they were made:
    /// each one's id, what an error in it is called, how its frame counts
    /// towards the probe reserve, and what the debug information says of
    /// it.
    handed: Vec<(FuncId, String, Frame, debug::FnDebug)>,
    /// What the debug information says of the functions defined, in the
    /// order they were made (decision 227).
    debug: Vec<debug::FnDebug>,
    /// Surveying (`survey`): a function with a construct the backend does
    /// not compile is recorded in `missing` and given a body that stops.
    survey: bool,
    missing: Vec<(String, String)>,
    /// Compiling the runtime in Sisu (decision 336): no frame counted, no
    /// yield point, no trace line, and no value walked by a type table.
    pub library: bool,
    /// The types of the values each `dyn` type holds, as the compiled
    /// code makes them (`dyn T(...)`).
    carried: fx::FxMap<Type, Vec<Type>>,
    /// The dispatchers of the calls through a `dyn`, one per method and
    /// type arguments, and where each is in the list.
    dispatchers: Vec<Dispatcher>,
    dispatcher_index: fx::FxMap<(String, Vec<Type>), usize>,
    /// Whether the dispatchers are written: from then on no `dyn` may be
    /// made and no call through one compiled.
    dispatched: bool,
    names: usize,
    pub compiled: usize,
    pub statements: usize,
    pub checks: Checks,
    /// The words of environment the largest `own` closure compiled needs,
    /// beyond what a function value holds: the program is compiled again.
    pub env_needed: u32,
}

/// What a call adds to the callee's frame beyond what Cranelift reports
/// as its size: the return address, the saved frame pointer, and the
/// arguments passed on the stack.
const FRAME_EXTRA: i64 = 256;

/// How a function's frame counts toward what a task's stack keeps below
/// its probe limit (decisions 203 and 234).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Frame {
    /// Its entry probes the stack, and nothing but the top of its frame
    /// (the saved registers and the spill slots, above its stack slots) is
    /// written before the probe: only that top may lie below the limit.
    Probed,
    /// It has no probe (a dispatcher, a function's thunk): its whole frame
    /// may lie below the limit, on top of its callee's top.
    Unprobed,
    /// It never runs on a task's stack (C's `main`, a stub that stops).
    Outside,
}

/// The `Sisu` signature: the result's address and each argument's, and
/// the status.
pub(crate) fn sisu_signature(call_conv: CallConv, params: u32) -> Signature {
    let mut sig = Signature::new(call_conv);
    for _ in 0..=params {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.returns.push(AbiParam::new(types::I8));
    sig
}

/// How a plain value of an exported function's signature goes between C's
/// register and the word Sisu's convention keeps it in (decision 785).
#[derive(Clone, Copy)]
enum Plain {
    /// A word as it is: `int`, `u64`, a raw pointer.
    Word,
    /// A narrower signed integer, sign-extended.
    Signed,
    /// A `bool`, a `char` or a narrower unsigned integer, zero-extended.
    Unsigned,
    F64,
    F32,
}

/// The C type of a plain value an exported function takes or gives
/// (stage 1's `E0991` admits the rest of its signature: a `@repr(c)`
/// struct, passed by a raw pointer to it), and how it becomes a word.
fn export_type(t: &Type) -> Option<(ir::Type, Plain)> {
    Some(match t {
        Type::Int => (types::I64, Plain::Word),
        Type::Float => (types::F64, Plain::F64),
        Type::Bool => (types::I8, Plain::Unsigned),
        Type::Char => (types::I32, Plain::Unsigned),
        Type::Named(n, _) => match n.as_str() {
            "u64" | "i64" | "ptr" | "ptr_mut" => (types::I64, Plain::Word),
            "i8" => (types::I8, Plain::Signed),
            "i16" => (types::I16, Plain::Signed),
            "i32" => (types::I32, Plain::Signed),
            "u8" => (types::I8, Plain::Unsigned),
            "u16" => (types::I16, Plain::Unsigned),
            "u32" => (types::I32, Plain::Unsigned),
            "f32" => (types::F32, Plain::F32),
            _ => return None,
        },
        _ => return None,
    })
}

/// The function a call of a trait's method on a `dyn` calls: it reads the
/// type of the value the `dyn` holds and calls what the call runs for that
/// type, with `self` the value's address (`sisu-interp/src/dispatch.rs`,
/// "a `dyn` value dispatches on the type it carries"). Its parameters are
/// the result's address, the address of the `dyn`'s two words, and the
/// other arguments' addresses.
struct Dispatcher {
    name: String,
    /// `Self`, the `dyn` type, then the other type arguments of the call.
    tys: Vec<Type>,
    id: FuncId,
    /// The types of the arguments after `self`, as the calls pass them,
    /// and of the result.
    arg_tys: Vec<Type>,
    ret_ty: Type,
    /// For each type the `dyn` holds values of: its number and what the
    /// call runs for it.
    cases: Vec<(Type, u32, Target)>,
}

impl<'p> Codegen<'p> {
    /// `library`: the runtime in Sisu, each of whose functions is a section
    /// of its own, so that a program's link leaves out those it does not
    /// reach (decision 682).
    fn new(
        prog: &'p sir::Program,
        env_words: u32,
        release: bool,
        library: bool,
    ) -> Result<Codegen<'p>, Error> {
        let mut flags = settings::builder();
        let set = |f: &mut settings::Builder, k: &str, v: &str| {
            f.set(k, v).map_err(|e| Error::internal(format!("the setting `{k}`: {e}")))
        };
        // The release profile is Cranelift's optimisations (decision 232).
        set(&mut flags, "opt_level", if release { "speed" } else { "none" })?;
        set(&mut flags, "is_pic", "true")?;
        // The runtime's words the code reaches by symbol are thread-local
        // (decision 332).
        set(&mut flags, "tls_model", "elf_gd")?;
        set(&mut flags, "enable_verifier", if cfg!(debug_assertions) { "true" } else { "false" })?;
        let isa = cranelift_native::builder()
            .map_err(|e| Error::internal(format!("this machine: {e}")))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| Error::internal(format!("this machine: {e}")))?;
        let call_conv = isa.default_call_conv();
        let shared = isa.clone();
        let mut builder = ObjectBuilder::new(isa, "sisu_program", cranelift_module::default_libcall_names())
            .map_err(|e| Error::internal(format!("the object file: {e}")))?;
        builder.per_function_section(library);
        let mut module = ObjectModule::new(builder);
        // Each thread keeps its own count of nested calls, budget, stack
        // limits and site (decision 332).
        let mut import = |name: &str| {
            module.declare_data(name, Linkage::Import, true, true).map_err(|e| Error::internal(e.to_string()))
        };
        let depth = import(sym::DEPTH)?;
        let budget = import(sym::BUDGET)?;
        let stack = import(sym::STACK)?;
        let site = import(sym::SITE)?;
        let mut fns = fx::FxMap::default();
        for f in &prog.functions {
            fns.entry(f.name.as_str()).or_insert(f);
        }
        Ok(Codegen {
            prog,
            fns,
            module,
            types: Types::new(prog, env_words),
            call_conv,
            instances: fx::FxMap::default(),
            queue: VecDeque::new(),
            runtime: fx::FxMap::default(),
            c_functions: fx::FxMap::default(),
            depth,
            budget,
            stack,
            site,
            max_top: 0,
            max_unprobed: 0,
            texts: fx::FxMap::default(),
            descs: fx::FxMap::default(),
            glue_ids: Vec::new(),
            thunks: fx::FxMap::default(),
            foreign: foreign_objects(prog, &reachable(prog)),
            fbctx: FunctionBuilderContext::new(),
            release,
            isa: shared,
            jobs: None,
            spares: None,
            handed: Vec::new(),
            debug: Vec::new(),
            survey: false,
            library: false,
            missing: Vec::new(),
            carried: fx::FxMap::default(),
            dispatchers: Vec::new(),
            dispatcher_index: fx::FxMap::default(),
            dispatched: false,
            names: 0,
            compiled: 0,
            statements: 0,
            checks: Checks::default(),
            env_needed: 0,
        })
    }

    /// The instance of `name` for these (concrete) type arguments,
    /// declared and queued to be compiled the first time it is asked for.
    /// A context to write a function's IR in, as `make_context` makes one,
    /// in a function the threads compiled and gave back when there is one.
    fn context(&mut self) -> Context {
        let mut ctx = match self.spares.as_ref().and_then(|r| r.try_recv().ok()) {
            Some(func) => Context::for_function(func),
            None => Context::new(),
        };
        ctx.func.signature.call_conv = self.call_conv;
        ctx
    }

    /// The program's function named `name`.
    pub(crate) fn function(&self, name: &str) -> Option<&'p sir::Function> {
        self.fns.get(name).copied()
    }

    /// The function `name`, without type arguments, under the global symbol
    /// `symbol` of a runtime library (decision 336), or of a program's
    /// `@export` (decision 683).
    fn export_instance(&mut self, name: &str, symbol: &str) -> Result<FuncId, Error> {
        let f = self
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let sig = sisu_signature(self.call_conv, f.param_count);
        let id = self
            .module
            .declare_function(symbol, Linkage::Export, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        self.instances.insert((name.to_string(), Vec::new()), id);
        self.queue.push_back((name.to_string(), Vec::new(), id));
        Ok(id)
    }

    /// A program's `@export("name")` (decision 785): the function under a
    /// local symbol of Sisu's convention, which `main`'s calls call, and the
    /// global symbol `symbol` of C's own convention over plain values, a
    /// wrapper compiled at once that writes each argument where Sisu's
    /// convention reads it, calls the function and gives back its result as
    /// C does; a panic that leaves the function stops the program with its
    /// report (`sisu_rt_export_panic`), since no C caller can unwind.
    fn export_c(&mut self, name: &str, symbol: &str) -> Result<(), Error> {
        let f = self
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        // A value the function takes by value, which the SIR passes by a
        // reference to it, is named as the program wrote it, and the error
        // names the function (decision 864).
        let plain = |t: &Type, what: &str| {
            export_type(t).ok_or_else(|| {
                let shown = match t {
                    Type::Ref(inner, _) => inner,
                    t => t,
                };
                Error::unsupported(format!(
                    "a `{}` by value ({what} of the exported function `{symbol}`)",
                    sir::write::ty(shown)
                ))
                .in_function(name)
            })
        };
        let mut params = Vec::new();
        for l in f.locals.iter().skip(1).take(f.param_count as usize) {
            params.push(plain(&l.ty, "a parameter")?);
        }
        let ret = match f.ret() {
            Type::Unit => None,
            t => Some(plain(t, "the result")?),
        };
        let inner = self.instance(name, Vec::new())?;
        let mut sig = Signature::new(self.call_conv);
        sig.params.extend(params.iter().map(|(t, _)| AbiParam::new(*t)));
        sig.returns.extend(ret.iter().map(|(t, _)| AbiParam::new(*t)));
        let id = self
            .module
            .declare_function(symbol, Linkage::Export, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        let failed = self.runtime(sym::EXPORT_PANIC)?;
        let mut ctx = self.context();
        ctx.func.signature = sig;
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let slot = |b: &mut FunctionBuilder<'_>| {
                b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3))
            };
            let result = slot(&mut b);
            let slots: Vec<_> = params.iter().map(|_| slot(&mut b)).collect();
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let given = b.block_params(entry).to_vec();
            let mut args = vec![b.ins().stack_addr(types::I64, result, 0)];
            for (((_, kind), v), s) in params.iter().zip(given).zip(&slots) {
                let word = match kind {
                    Plain::Word => v,
                    Plain::Signed => b.ins().sextend(types::I64, v),
                    Plain::Unsigned => b.ins().uextend(types::I64, v),
                    Plain::F64 => b.ins().bitcast(types::I64, MemFlags::new(), v),
                    Plain::F32 => {
                        let bits = b.ins().bitcast(types::I32, MemFlags::new(), v);
                        b.ins().uextend(types::I64, bits)
                    }
                };
                b.ins().stack_store(word, *s, 0);
                args.push(b.ins().stack_addr(types::I64, *s, 0));
            }
            let inner_ref = self.module.declare_func_in_func(inner, b.func);
            let call = b.ins().call(inner_ref, &args);
            let status = b.inst_results(call)[0];
            let fail = b.create_block();
            let done = b.create_block();
            b.ins().brif(status, fail, &[], done, &[]);
            b.set_cold_block(fail);
            b.switch_to_block(fail);
            let failed_ref = self.module.declare_func_in_func(failed, b.func);
            b.ins().call(failed_ref, &[]);
            b.ins().trap(ir::TrapCode::unwrap_user(1));
            b.switch_to_block(done);
            match ret {
                None => {
                    b.ins().return_(&[]);
                }
                Some((t, kind)) => {
                    let word = b.ins().stack_load(types::I64, result, 0);
                    let v = match kind {
                        Plain::Word => word,
                        Plain::Signed | Plain::Unsigned => b.ins().ireduce(t, word),
                        Plain::F64 => b.ins().bitcast(types::F64, MemFlags::new(), word),
                        Plain::F32 => {
                            let low = b.ins().ireduce(types::I32, word);
                            b.ins().bitcast(types::F32, MemFlags::new(), low)
                        }
                    };
                    b.ins().return_(&[v]);
                }
            }
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        self.define(id, ctx, symbol.to_string(), Frame::Unprobed, debug::FnDebug::new(id, symbol.to_string()))
    }

    pub(crate) fn instance(&mut self, name: &str, targs: Vec<Type>) -> Result<FuncId, Error> {
        let key = (name.to_string(), targs);
        if let Some(&id) = self.instances.get(&key) {
            return Ok(id);
        }
        let f = self
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let sig = sisu_signature(self.call_conv, f.param_count);
        self.names += 1;
        let symbol = format!("sisu${}${}", sanitize(name), self.names);
        let id = self
            .module
            .declare_function(&symbol, Linkage::Local, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        self.instances.insert(key.clone(), id);
        self.queue.push_back((key.0, key.1, id));
        Ok(id)
    }

    /// Records that a `dyn` of type `dyn_ty` holds a value of type
    /// `value_ty`.
    pub(crate) fn carry(&mut self, dyn_ty: &Type, value_ty: &Type) -> Result<(), Error> {
        let held = self.carried.entry(dyn_ty.clone()).or_default();
        if !held.contains(value_ty) {
            if self.dispatched {
                return Err(Error::internal("a `dyn` made after the dispatchers were written"));
            }
            held.push(value_ty.clone());
        }
        Ok(())
    }

    /// What a call of the intrinsic `name` tells the dispatchers (decision
    /// 209): a handle made on an object may hold it, and the host's
    /// sockets are what the network's `dyn`s hold.
    pub(crate) fn note_intrinsic(&mut self, name: &str, targs: &[Type]) -> Result<(), Error> {
        let dyn_of = |t: &str| Type::Dyn(t.to_string(), Vec::new());
        let named = |t: &str| Type::Named(t.to_string(), Vec::new());
        match name {
            "cap.new" => match targs {
                [object, handle @ Type::Named(..)] => self.carry(handle, object),
                _ => Err(Error::internal("`cap.new` without its object's and handle's types")),
            },
            // The runtime finds the sockets' types by name in the table.
            "host.net.connect" | "host.net.accept" => {
                self.types.id(&named("HostStream"))?;
                self.carry(&dyn_of("Stream"), &named("HostStream"))
            }
            "host.net.listen" => {
                self.types.id(&named("HostListener"))?;
                self.carry(&dyn_of("Accept"), &named("HostListener"))
            }
            _ => Ok(()),
        }
    }

    /// The dispatcher of `name` (a trait's method) for these type
    /// arguments, whose `Self` is a `dyn`: declared the first time it is
    /// asked for, and written when the program is complete.
    pub(crate) fn dispatcher(
        &mut self,
        name: &str,
        tys: Vec<Type>,
        arg_tys: Vec<Type>,
        ret_ty: Type,
    ) -> Result<FuncId, Error> {
        let key = (name.to_string(), tys);
        if let Some(&i) = self.dispatcher_index.get(&key) {
            return Ok(self.dispatchers[i].id);
        }
        if self.dispatched {
            return Err(Error::internal(
                "a call through a `dyn` compiled after the dispatchers were written",
            ));
        }
        let sig = sisu_signature(self.call_conv, arg_tys.len() as u32 + 1);
        self.names += 1;
        let symbol = format!("sisu$dyn${}${}", sanitize(name), self.names);
        let id = self
            .module
            .declare_function(&symbol, Linkage::Local, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        self.dispatcher_index.insert(key.clone(), self.dispatchers.len());
        self.dispatchers.push(Dispatcher { name: key.0, tys: key.1, id, arg_tys, ret_ty, cases: Vec::new() });
        Ok(id)
    }

    /// Compiles everything the program needs: the queued instances, then
    /// what each dispatcher runs for each type its `dyn` holds a value of
    /// (which may need more instances, which may make more `dyn`s), until
    /// nothing is left; then writes the dispatchers.
    fn complete(&mut self) -> Result<(), Error> {
        if self.dispatched {
            return self.drain();
        }
        loop {
            self.drain()?;
            let mut more = false;
            for i in 0..self.dispatchers.len() {
                let self_ty = self.dispatchers[i].tys.first().cloned().unwrap_or(Type::Unit);
                let held = self.carried.get(&self_ty).cloned().unwrap_or_default();
                for value_ty in held {
                    if self.dispatchers[i].cases.iter().any(|(t, _, _)| *t == value_ty) {
                        continue;
                    }
                    let name = self.dispatchers[i].name.clone();
                    let mut tys = self.dispatchers[i].tys.clone();
                    if let Some(t) = tys.first_mut() {
                        *t = value_ty.clone();
                    }
                    let (target, _) = self.resolve_trait(&name, tys).map_err(|e| e.in_function(&name))?;
                    match &target {
                        Target::Fn(f, targs) => {
                            self.instance(f, targs.clone())?;
                        }
                        Target::Intrinsic(i, targs) => self.note_intrinsic(i, targs)?,
                        Target::Dyn(..) => return Err(Error::unsupported("a `dyn` that holds a `dyn`")),
                    }
                    let number = self.types.id(&value_ty)?;
                    self.dispatchers[i].cases.push((value_ty, number, target));
                    more = true;
                }
            }
            if !more {
                break;
            }
        }
        for i in 0..self.dispatchers.len() {
            self.define_dispatcher(i)?;
        }
        self.dispatched = true;
        // The drop glue of the types the dispatchers named.
        self.drain()
    }

    /// Writes dispatcher `i`: a switch on the number of the type of the
    /// value the `dyn` holds, and for each type a call of what the call
    /// runs for it.
    fn define_dispatcher(&mut self, i: usize) -> Result<(), Error> {
        let (id, n, ret_ty, arg_tys, cases) = {
            let d = &self.dispatchers[i];
            (d.id, d.arg_tys.len() + 1, d.ret_ty.clone(), d.arg_tys.clone(), d.cases.clone())
        };
        // What each case passes as `self`, and how: the value's address for
        // a method that borrows it, a copy of the value for one that takes
        // it (a `Copy` type's, decision 59), and for an intrinsic the value's
        // address as a reference, with the call's descriptor.
        // A method owns its arguments: the copy of the value a by-value
        // `self` takes, when it holds a capability handle, is counted with
        // its type (decision 237); the call counts the other arguments.
        enum Call {
            Fn { callee: FuncId, by_value: Option<u32>, count: Option<u32> },
            Intrinsic { desc: DataId },
        }
        let mut calls = Vec::new();
        for (value_ty, _, target) in &cases {
            calls.push(match target {
                Target::Fn(f, targs) => {
                    let func = self
                        .function(f)
                        .ok_or_else(|| Error::internal(format!("the program has no function `{f}`")))?;
                    let first = match func.locals.get(1) {
                        Some(l) => self.concrete(&l.ty, targs)?,
                        None => Type::Unit,
                    };
                    let by_value =
                        if matches!(first, Type::Ref(..)) { None } else { Some(self.types.size(value_ty)?) };
                    let count = match by_value.is_some() && self.types.counted(value_ty)? {
                        true => Some(self.types.id(value_ty)?),
                        false => None,
                    };
                    Call::Fn { callee: self.instance(f, targs.clone())?, by_value, count }
                }
                Target::Intrinsic(name, targs) => {
                    let Some(intrinsic) = Intrinsic::from_name(name) else {
                        return Err(Error::unsupported(format!("the intrinsic `{name}`")));
                    };
                    let mut desc =
                        vec![intrinsic as u32, n as u32, targs.len() as u32, self.types.id(&ret_ty)?];
                    desc.push(self.types.id(&Type::Ref(Box::new(value_ty.clone()), false))?);
                    for t in &arg_tys {
                        desc.push(self.types.id(t)?);
                    }
                    for t in targs {
                        desc.push(self.types.id(t)?);
                    }
                    Call::Intrinsic { desc: self.desc(desc)? }
                }
                Target::Dyn(..) => return Err(Error::unsupported("a `dyn` that holds a `dyn`")),
            });
        }
        let intrinsic_fn = if calls.iter().any(|c| matches!(c, Call::Intrinsic { .. })) {
            Some(self.runtime(sym::INTRINSIC)?)
        } else {
            None
        };
        let retain_fn = if calls.iter().any(|c| matches!(c, Call::Fn { count: Some(_), .. })) {
            Some(self.runtime(sym::RETAIN)?)
        } else {
            None
        };
        let mut ctx = self.context();
        ctx.func.signature = sisu_signature(self.call_conv, n as u32);
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            // A function's references to other functions and to data are
            // made once each, at their first use, as everywhere (the
            // translation of LIR makes them so, decision 283).
            let mut funcs: fx::FxMap<FuncId, ir::FuncRef> = fx::FxMap::default();
            let mut func_ref = |module: &mut ObjectModule, b: &mut FunctionBuilder<'_>, id: FuncId| {
                *funcs.entry(id).or_insert_with(|| module.declare_func_in_func(id, b.func))
            };
            let mut datas: fx::FxMap<DataId, ir::GlobalValue> = fx::FxMap::default();
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let params: Vec<ir::Value> = b.block_params(entry).to_vec();
            let (ret, dyn_words) = (params[0], params[1]);
            let flags = MemFlags::trusted();
            let number = b.ins().load(types::I64, flags, dyn_words, 0);
            let value = b.ins().load(types::I64, flags, dyn_words, 8);
            let mut sw = Switch::new();
            let mut blocks = Vec::new();
            for (_, type_number, _) in &cases {
                let blk = b.create_block();
                sw.set_entry(u128::from(*type_number), blk);
                blocks.push(blk);
            }
            let default = b.create_block();
            sw.emit(&mut b, number, default);
            b.switch_to_block(default);
            b.ins().trap(ir::TrapCode::unwrap_user(3));
            for (blk, call) in blocks.into_iter().zip(&calls) {
                b.switch_to_block(blk);
                let slot = |b: &mut FunctionBuilder<'_>, size: u32| {
                    let ss = b.create_sized_stack_slot(ir::StackSlotData::new(
                        ir::StackSlotKind::ExplicitSlot,
                        size.max(8),
                        3,
                    ));
                    b.ins().stack_addr(types::I64, ss, 0)
                };
                let status = match call {
                    Call::Fn { callee, by_value, count } => {
                        let me = match by_value {
                            None => {
                                let s = slot(&mut b, 8);
                                b.ins().store(flags, value, s, 0);
                                s
                            }
                            Some(size) => {
                                let s = slot(&mut b, *size);
                                for k in 0..size / 8 {
                                    let w = b.ins().load(types::I64, flags, value, (k * 8) as i32);
                                    b.ins().store(flags, w, s, (k * 8) as i32);
                                }
                                s
                            }
                        };
                        if let (Some(rf), Some(t)) = (retain_fn, count) {
                            let tv = b.ins().iconst(types::I32, i64::from(*t));
                            let r = func_ref(&mut self.module, &mut b, rf);
                            b.ins().call(r, &[me, tv]);
                        }
                        let r = func_ref(&mut self.module, &mut b, *callee);
                        let mut args = vec![ret, me];
                        args.extend(params.iter().skip(2).copied());
                        let c = b.ins().call(r, &args);
                        b.inst_results(c)[0]
                    }
                    Call::Intrinsic { desc } => {
                        let me = slot(&mut b, 8);
                        b.ins().store(flags, value, me, 0);
                        let arr = slot(&mut b, 8 * n as u32);
                        b.ins().store(flags, me, arr, 0);
                        for (k, a) in params.iter().skip(2).enumerate() {
                            b.ins().store(flags, *a, arr, (8 * (k + 1)) as i32);
                        }
                        let gv = *datas
                            .entry(*desc)
                            .or_insert_with(|| self.module.declare_data_in_func(*desc, b.func));
                        let d = b.ins().symbol_value(types::I64, gv);
                        let f = intrinsic_fn.ok_or_else(|| Error::internal("no intrinsic entry point"))?;
                        let r = func_ref(&mut self.module, &mut b, f);
                        let c = b.ins().call(r, &[d, ret, arr]);
                        b.inst_results(c)[0]
                    }
                };
                b.ins().return_(&[status]);
            }
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        let what = format!("the dispatcher of `{}`", self.dispatchers[i].name);
        let dbg = debug::FnDebug::new(id, format!("{}#dispatch", self.dispatchers[i].name));
        self.define(id, ctx, what, Frame::Unprobed, dbg)
    }

    /// Defines function `id` with the body in `ctx`: compiled here, or
    /// handed to the threads that compile (decision 219), whose code
    /// [`Codegen::place`] defines. `what` names the function in an error;
    /// `frame`: how its frame counts towards the probe reserve.
    fn define(
        &mut self,
        id: FuncId,
        mut ctx: Context,
        what: String,
        frame: Frame,
        dbg: debug::FnDebug,
    ) -> Result<(), Error> {
        workers::dump_clif(&ctx.func);
        if let Some(jobs) = &self.jobs {
            let job = workers::Job { order: self.handed.len(), id, func: ctx.func };
            if jobs.send(job).is_err() {
                return Err(Error::internal(format!("{what}: the threads that compile stopped")));
            }
            self.handed.push((id, what, frame, dbg));
            return Ok(());
        }
        self.module.define_function(id, &mut ctx).map_err(|e| Error::internal(format!("{what}: {e:?}")))?;
        if let Some(code) = ctx.compiled_code() {
            let top = code.frame_size.saturating_sub(workers::slots_size(&ctx.func));
            self.count_frame(frame, code.frame_size, top);
            let size = code.buffer.data().len() as u32;
            self.debug.push(dbg.compiled(size, workers::Compiled::of(code, &*self.isa)));
        }
        Ok(())
    }

    /// Defines the functions the threads compiled, in the order they were
    /// made, so the object file is the same however the threads ran.
    fn place(&mut self, done: Vec<(usize, Result<workers::Code, String>)>) -> Result<(), Error> {
        let mut codes: Vec<Option<Result<workers::Code, String>>> =
            std::iter::repeat_with(|| None).take(self.handed.len()).collect();
        for (order, code) in done {
            if let Some(c) = codes.get_mut(order) {
                *c = Some(code);
            }
        }
        for ((id, what, frame, dbg), code) in std::mem::take(&mut self.handed).into_iter().zip(codes) {
            let code = match code {
                Some(Ok(code)) => code,
                Some(Err(e)) => return Err(Error::internal(format!("{what}: {e}"))),
                None => return Err(Error::internal(format!("{what}: no code came back"))),
            };
            self.module
                .define_function_bytes(id, code.alignment, &code.bytes, &code.relocs)
                .map_err(|e| Error::internal(format!("{what}: {e:?}")))?;
            self.count_frame(frame, code.frame_size, code.top);
            self.debug.push(dbg.compiled(code.bytes.len() as u32, code.debug));
        }
        Ok(())
    }

    /// Counts a compiled function's frame, of `size` bytes whose `top` its
    /// entry writes before its probe, toward the probe reserve.
    fn count_frame(&mut self, frame: Frame, size: u32, top: u32) {
        match frame {
            Frame::Probed => self.max_top = self.max_top.max(top),
            Frame::Unprobed => self.max_unprobed = self.max_unprobed.max(size),
            Frame::Outside => {}
        }
    }

    /// Everything but `main`: the instances the entry reaches, the
    /// dispatchers and the drop glue; the entry's id, `main`'s flags and
    /// the number of the entry's result type.
    fn generate(&mut self, entry: &sir::Entry) -> Result<(FuncId, u32, u32), Error> {
        // Each `@export("name")` function under its global symbol, of
        // Sisu's convention (decision 683), first, so that `main`'s calls of
        // it call that symbol.
        for (symbol, function) in &self.prog.exports {
            self.export_c(function, symbol)?;
        }
        let main_id = self.instance(&entry.function, Vec::new())?;
        self.complete()?;
        let ret = self.function(&entry.function).map(|f| f.ret().clone()).unwrap_or(Type::Unit);
        let ret_ty = self.types.id(&self.concrete(&ret, &[])?)?;
        self.complete()?;
        let mut flags = if entry.returns_result { abi::ENTRY_RESULT } else { 0 };
        if entry.takes_system {
            flags |= abi::ENTRY_SYSTEM;
        }
        // The debug profile checks raw memory (decision 543).
        if !self.release {
            flags |= abi::ENTRY_CHECKED;
        }
        // The runtime profile `single` has no workers for `std.par`
        // (decision 783).
        if self.prog.runtime.as_deref() == Some("single") {
            flags |= abi::ENTRY_SINGLE;
        }
        Ok((main_id, flags, ret_ty))
    }

    /// Compiles every queued instance, and the drop glue the type table
    /// names, until nothing is left.
    fn drain(&mut self) -> Result<(), Error> {
        loop {
            while self.glue_ids.len() < self.types.glue.len() {
                let (name, targs) = self.types.glue[self.glue_ids.len()].clone();
                let id = self.instance(&name, targs)?;
                self.glue_ids.push(id);
            }
            let Some((name, targs, id)) = self.queue.pop_front() else {
                if self.glue_ids.len() == self.types.glue.len() {
                    return Ok(());
                }
                continue;
            };
            match self.compile_instance(&name, &targs, id).map_err(|e| e.in_function(&name)) {
                Err(Error::Unsupported { what, function }) if self.survey => {
                    self.missing.push((what, function));
                    self.stub(&name, id)?;
                }
                other => other?,
            }
        }
    }

    fn compile_instance(&mut self, name: &str, targs: &[Type], id: FuncId) -> Result<(), Error> {
        let f = self
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let mut ctx: Context = self.context();
        ctx.func.signature = sisu_signature(self.call_conv, f.param_count);
        let mut fbctx = std::mem::take(&mut self.fbctx);
        let result = {
            let b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            func::translate(self, b, f, targs)
        };
        // A translation that stopped halfway leaves the builder's context
        // in use: the next function starts from a fresh one.
        self.fbctx = if result.is_ok() { fbctx } else { FunctionBuilderContext::new() };
        let vars = result?;
        let mut shown = name.to_string();
        if !targs.is_empty() {
            let args: Vec<String> = targs.iter().map(sir::write::ty).collect();
            shown = format!("{shown}[{}]", args.join(", "));
        }
        let mut dbg = debug::FnDebug::new(id, shown);
        dbg.source = Some(f.source);
        dbg.line = f.loc.map_or(0, |l| l.line);
        dbg.vars = vars;
        self.define(id, ctx, format!("Cranelift rejected `{name}`"), Frame::Probed, dbg)?;
        self.compiled += 1;
        Ok(())
    }

    /// The named function `name` as a value: the function that calls it
    /// with an environment first (which it ignores), and the value's type.
    pub(crate) fn fn_value(&mut self, name: &str) -> Result<(FuncId, Type), Error> {
        if let Some(t) = self.thunks.get(name) {
            return Ok(t.clone());
        }
        let f = self
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        if !f.type_params.is_empty() {
            return Err(Error::unsupported(format!("the generic function `{name}` as a value")));
        }
        let n = f.param_count;
        let mut params = Vec::new();
        for l in f.locals.iter().skip(1).take(n as usize) {
            params.push(self.concrete(&l.ty, &[])?);
        }
        let ty = Type::Fn(params, Box::new(self.concrete(f.ret(), &[])?), f.pure);
        let target = self.instance(name, Vec::new())?;
        let sig = sisu_signature(self.call_conv, n + 1);
        self.names += 1;
        let symbol = format!("sisu${}$value${}", sanitize(name), self.names);
        let id = self
            .module
            .declare_function(&symbol, Linkage::Local, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        let mut ctx = self.context();
        ctx.func.signature = sig;
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            let params: Vec<ir::Value> = b.block_params(block).to_vec();
            let target_ref = self.module.declare_func_in_func(target, b.func);
            // The result's address, then the arguments without the
            // environment.
            let mut args = vec![params[0]];
            args.extend(params.iter().skip(2).copied());
            let call = b.ins().call(target_ref, &args);
            let st = b.inst_results(call)[0];
            b.ins().return_(&[st]);
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        self.define(
            id,
            ctx,
            format!("{name} as a value"),
            Frame::Unprobed,
            debug::FnDebug::new(id, format!("{name}#thunk")),
        )?;
        self.thunks.insert(name.to_string(), (id, ty.clone()));
        Ok((id, ty))
    }

    /// A body that stops the program, for a function the survey could not
    /// compile.
    fn stub(&mut self, name: &str, id: FuncId) -> Result<(), Error> {
        let params = self.function(name).map(|f| f.param_count).unwrap_or(0);
        let mut ctx = self.context();
        ctx.func.signature = sisu_signature(self.call_conv, params);
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            b.ins().trap(ir::TrapCode::unwrap_user(2));
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        self.define(id, ctx, name.to_string(), Frame::Outside, debug::FnDebug::new(id, name.to_string()))
    }

    /// A runtime entry point, declared once with its signature.
    pub(crate) fn runtime(&mut self, name: &'static str) -> Result<FuncId, Error> {
        if let Some(&id) = self.runtime.get(name) {
            return Ok(id);
        }
        let (i64_, i32_, i8_, f64_) = (types::I64, types::I32, types::I8, types::F64);
        let (params, returns): (&[ir::Type], &[ir::Type]) = match name {
            sym::START => (&[i32_, i64_, i64_, i32_, i32_, i64_, i64_, i64_, i64_, i64_], &[i32_]),
            sym::YIELD => (&[], &[i8_]),
            sym::ARENA_SLOT => (&[i64_, i64_, i32_, i64_], &[i8_]),
            sym::SHARED_TOUCH => (&[i64_], &[]),
            sym::FRAME => (&[i64_, i64_, i64_, i64_, i64_, i64_], &[]),
            sym::ABORT_UNWIND => (&[], &[]),
            sym::PANIC_OVERFLOW => (&[i32_, i64_, i64_], &[i8_]),
            sym::PANIC_INDEX => (&[i64_, i64_, i64_], &[i8_]),
            sym::PANIC_STACK => (&[i64_, i64_], &[i8_]),
            sym::RETAIN => (&[i64_, i32_], &[]),
            sym::PANIC_UNREACHABLE => (&[], &[i8_]),
            sym::INT_OP => (&[i32_, i64_, i64_, i64_], &[i8_]),
            sym::FLOAT_OP => (&[i32_, f64_, f64_], &[f64_]),
            sym::ALLOC => (&[i64_], &[i64_]),
            sym::TASK_ENV => (&[i64_, i32_], &[i64_]),
            sym::FREE | sym::TRACE_RELEASE => (&[i64_], &[]),
            sym::DROP => (&[i64_, i32_], &[i8_]),
            sym::COPY => (&[i64_, i64_, i32_], &[]),
            sym::BINARY => (&[i32_, i64_, i64_, i32_, i64_, i32_], &[i8_]),
            // The runtime in Sisu's, of Sisu's convention (decisions 336 and
            // 350): the result's address, each argument's, the status.
            sym::BYTES_EQ | sym::TEXT_PUSH | sym::TEXT_FROM => (&[i64_, i64_, i64_, i64_], &[i8_]),
            sym::TEXT_PUSH_INT => (&[i64_, i64_, i64_], &[i8_]),
            sym::LIST_RESERVE => (&[i64_, i64_], &[]),
            sym::TABLE_FIND => (&[i64_, i64_, i32_, i32_], &[i64_]),
            sym::INTRINSIC => (&[i64_, i64_, i64_], &[i8_]),
            sym::SIZED => (&[i32_, i32_, i32_, i64_, i64_, i64_], &[i8_]),
            sym::WIDE => (&[i32_, i32_, i64_, i32_, i64_, i32_, i64_], &[i8_]),
            sym::TABLE => (&[i32_, i64_, i64_, i64_], &[i8_]),
            sym::DICT_SLOT | sym::TABLE_TAKE => (&[i64_, i64_, i32_, i32_, i64_], &[i8_]),
            sym::FLUSH => (&[], &[]),
            sym::C_CHECKED => (&[], &[i8_]),
            sym::THREAD_SLOT => (&[], &[i64_]),
            sym::SWITCH => (&[i64_, i64_], &[]),
            sym::EXPORT_PANIC => (&[], &[]),
            _ => return Err(Error::internal(format!("an unknown runtime entry point `{name}`"))),
        };
        let mut sig = Signature::new(self.call_conv);
        sig.params.extend(params.iter().map(|t| AbiParam::new(*t)));
        sig.returns.extend(returns.iter().map(|t| AbiParam::new(*t)));
        let id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        self.runtime.insert(name, id);
        Ok(id)
    }

    /// The foreign function `name` (a C function of an `extern` block),
    /// declared with the C signature its first call gives; a call that
    /// gives another is a construct the backend does not compile.
    pub(crate) fn c_function(
        &mut self,
        name: &str,
        params: &[ir::Type],
        returns: &[ir::Type],
    ) -> Result<FuncId, Error> {
        if let Some((id, p, r)) = self.c_functions.get(name) {
            if p.as_slice() != params || r.as_slice() != returns {
                return Err(Error::unsupported(format!(
                    "two calls of the foreign function `{name}` with different C signatures"
                )));
            }
            return Ok(*id);
        }
        let mut sig = Signature::new(self.call_conv);
        sig.params.extend(params.iter().map(|t| AbiParam::new(*t)));
        sig.returns.extend(returns.iter().map(|t| AbiParam::new(*t)));
        let id = self
            .module
            .declare_function(name, Linkage::Import, &sig)
            .map_err(|e| Error::unsupported(format!("the foreign function `{name}`: {e}")))?;
        self.c_functions.insert(name.to_string(), (id, params.to_vec(), returns.to_vec()));
        Ok(id)
    }

    /// Read-only bytes, once per content.
    pub(crate) fn text(&mut self, bytes: &[u8]) -> Result<DataId, Error> {
        if let Some(&id) = self.texts.get(bytes) {
            return Ok(id);
        }
        let id =
            self.module.declare_anonymous_data(false, false).map_err(|e| Error::internal(e.to_string()))?;
        let mut d = DataDescription::new();
        // A text needs no terminator; one byte keeps an empty text's
        // address valid.
        let mut content = bytes.to_vec();
        content.push(0);
        d.define(content.into_boxed_slice());
        self.module.define_data(id, &d).map_err(|e| Error::internal(e.to_string()))?;
        self.texts.insert(bytes.to_vec(), id);
        Ok(id)
    }

    /// A call descriptor (`abi::CallDesc`) in the read-only data.
    pub(crate) fn desc(&mut self, words: Vec<u32>) -> Result<DataId, Error> {
        if let Some(&id) = self.descs.get(&words) {
            return Ok(id);
        }
        let id =
            self.module.declare_anonymous_data(false, false).map_err(|e| Error::internal(e.to_string()))?;
        let mut d = DataDescription::new();
        let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        // A word of 0 after an odd number of them, so that the runtime in
        // Sisu reads every word two at a time (decision 862).
        if words.len() % 2 == 1 {
            bytes.extend_from_slice(&[0; 4]);
        }
        d.define(bytes.into_boxed_slice());
        d.set_align(8);
        self.module.define_data(id, &d).map_err(|e| Error::internal(e.to_string()))?;
        self.descs.insert(words, id);
        Ok(id)
    }

    /// A type with the instance's type arguments put in, and the
    /// associated types of impls looked up.
    pub(crate) fn concrete(&self, t: &Type, targs: &[Type]) -> Result<Type, Error> {
        if settled(t) {
            return Ok(t.clone());
        }
        let t = t.subst(targs);
        self.resolve_assoc(&t)
    }

    fn resolve_assoc(&self, t: &Type) -> Result<Type, Error> {
        Ok(match t {
            Type::Assoc(base, tr, args, name) => {
                let base = self.resolve_assoc(base)?;
                let args: Vec<Type> = args.iter().map(|a| self.resolve_assoc(a)).collect::<Result<_, _>>()?;
                for imp in &self.prog.impls {
                    if imp.trait_name != *tr {
                        continue;
                    }
                    let mut binds = vec![None; imp.params as usize];
                    if !func::match_type(&imp.self_ty, &base, &mut binds)
                        || !imp.trait_args.iter().zip(&args).all(|(p, a)| func::match_type(p, a, &mut binds))
                    {
                        continue;
                    }
                    if let Some((_, at)) = imp.assoc.iter().find(|(n, _)| n == name) {
                        let binds: Vec<Type> = binds.into_iter().map(|b| b.unwrap_or(Type::Unit)).collect();
                        return self.resolve_assoc(&at.subst(&binds));
                    }
                }
                return Err(Error::unsupported(format!("the associated type `{}`", sir::write::ty(t))));
            }
            Type::Named(n, a) => {
                Type::Named(n.clone(), a.iter().map(|x| self.resolve_assoc(x)).collect::<Result<_, _>>()?)
            }
            Type::Tuple(a) => Type::Tuple(a.iter().map(|x| self.resolve_assoc(x)).collect::<Result<_, _>>()?),
            Type::Ref(x, m) => Type::Ref(Box::new(self.resolve_assoc(x)?), *m),
            Type::Fn(p, r, pure) => Type::Fn(
                p.iter().map(|x| self.resolve_assoc(x)).collect::<Result<_, _>>()?,
                Box::new(self.resolve_assoc(r)?),
                *pure,
            ),
            Type::Param(i) => {
                return Err(Error::internal(format!("the type parameter `${i}` has no argument")));
            }
            other => other.clone(),
        })
    }

    /// A call through a capability handle (`cap.forward`): the operation
    /// of the object behind it. A handle of a type the program makes no
    /// other objects for is on the host's object, whose operation is called
    /// directly (decision 194); one that may hold another object is the
    /// address of a cell holding the object as a `dyn` holds its value,
    /// and the call is dispatched on the object's type (decision 209). A
    /// `Lent` is its handle.
    pub(crate) fn forward(
        &mut self,
        name: &str,
        handle: &Type,
        mut tys: Vec<Type>,
        derefs: usize,
    ) -> Result<(Target, usize), Error> {
        if let Type::Named(n, a) = handle
            && n == "Lent"
            && let [inner] = a.as_slice()
        {
            if let Some(t) = tys.first_mut() {
                *t = inner.clone();
            }
            let (target, more) = self.resolve_trait(name, tys)?;
            return Ok((target, derefs + more));
        }
        let host = match handle {
            Type::Named(h, a) if a.is_empty() => {
                abi::CAPABILITIES.iter().find(|(c, _)| c == h).map(|(_, o)| *o)
            }
            _ => None,
        };
        let (Some(host), Type::Named(h, _)) = (host, handle) else {
            return Err(Error::unsupported(format!(
                "a call of `{name}` through a `{}`",
                sir::write::ty(handle)
            )));
        };
        if self.foreign.contains(h) {
            // The root handles of `main`'s `System` hold the host's objects.
            self.carry(handle, &Type::Named(host.to_string(), Vec::new()))?;
            return Ok((Target::Dyn(name.to_string(), tys), derefs));
        }
        if let Some(t) = tys.first_mut() {
            *t = Type::Named(host.to_string(), Vec::new());
        }
        let (target, _) = self.resolve_trait(name, tys)?;
        Ok((target, derefs))
    }

    /// What a call through a trait runs for these (concrete) type
    /// arguments, and how many references its `Self` is behind
    /// (`sisu-interp/src/dispatch.rs`, at compile time); a `dyn` `Self`
    /// is dispatched when the program runs, on the type of its value.
    pub(crate) fn resolve_trait(&mut self, name: &str, mut tys: Vec<Type>) -> Result<(Target, usize), Error> {
        let (trait_name, method) =
            name.rsplit_once('.').ok_or_else(|| Error::internal(format!("bad trait method `{name}`")))?;
        let mut self_ty =
            tys.first().cloned().ok_or_else(|| Error::internal(format!("`{name}` without `Self`")))?;
        let mut derefs = 0;
        while let Type::Ref(inner, _) = self_ty {
            self_ty = *inner;
            derefs += 1;
        }
        if let Some(t) = tys.first_mut() {
            *t = self_ty.clone();
        }
        // A `dyn` dispatches on the type of the value it holds, which only
        // the running program knows.
        if matches!(self_ty, Type::Dyn(..)) {
            return Ok((Target::Dyn(name.to_string(), tys), derefs));
        }
        let prog = self.prog;
        for imp in &prog.impls {
            if imp.trait_name != trait_name {
                continue;
            }
            let n = imp.trait_args.len();
            let mut binds: Vec<Option<Type>> = vec![None; imp.params as usize];
            if !func::match_type(&imp.self_ty, &self_ty, &mut binds)
                || !imp
                    .trait_args
                    .iter()
                    .zip(tys.iter().skip(1))
                    .all(|(p, t)| func::match_type(p, t, &mut binds))
            {
                continue;
            }
            let mut targs: Vec<Type> = binds.into_iter().map(|b| b.unwrap_or(Type::Unit)).collect();
            targs.extend(tys.iter().skip(1 + n).cloned());
            let target = match imp.methods.iter().find(|(m, _)| m == method) {
                Some((_, ImplTarget::Intrinsic(i))) if i == "cap.forward" => {
                    return self.forward(name, &self_ty, tys, derefs);
                }
                Some((_, ImplTarget::Fn(f))) => Target::Fn(f.clone(), targs),
                Some((_, ImplTarget::Intrinsic(i))) => Target::Intrinsic(i.clone(), targs),
                None => Target::Fn(name.to_string(), tys),
            };
            return Ok((target, derefs));
        }
        let structural = match (trait_name, method) {
            ("Display", "fmt") => "structural.display",
            ("Debug", "fmt_debug") => "structural.debug",
            ("Eq", "eq") => "structural.eq",
            ("PartialOrd", "partial_cmp") => "structural.partial_cmp",
            ("Ord", "cmp") => "structural.cmp",
            ("Hash", "hash") => "structural.hash",
            _ => {
                return Err(Error::internal(format!(
                    "no impl of `{trait_name}` for `{}`",
                    sir::write::ty(&self_ty)
                )));
            }
        };
        Ok((Target::Intrinsic(structural.into(), tys), derefs))
    }

    /// The C `main`: hands the program's tables and its entry to
    /// `sisu_rt_start`.
    fn define_main(&mut self, entry: FuncId, flags: u32, ret_ty: u32) -> Result<(), Error> {
        let table = abi::encode(&self.types.table()?);
        let types_id = self.text(&table)?;
        // The function table: the address of each glue instance.
        let fns_id =
            self.module.declare_anonymous_data(false, false).map_err(|e| Error::internal(e.to_string()))?;
        let mut d = DataDescription::new();
        let n = self.glue_ids.len();
        d.define(vec![0u8; (n.max(1)) * 8].into_boxed_slice());
        d.set_align(8);
        for (i, f) in self.glue_ids.clone().into_iter().enumerate() {
            let fref = self.module.declare_func_in_data(f, &mut d);
            d.write_function_addr((i * 8) as u32, fref);
        }
        self.module.define_data(fns_id, &d).map_err(|e| Error::internal(e.to_string()))?;

        let mut sig = Signature::new(self.call_conv);
        sig.params.push(AbiParam::new(types::I32));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I32));
        let main = self
            .module
            .declare_function("main", Linkage::Export, &sig)
            .map_err(|e| Error::internal(e.to_string()))?;
        let start = self.runtime(sym::START)?;
        let mut ctx = self.context();
        ctx.func.signature = sig;
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            let (argc, argv) = (b.block_params(block)[0], b.block_params(block)[1]);
            // References made at their first use, as everywhere.
            let entry_ref = self.module.declare_func_in_func(entry, b.func);
            let entry_addr = b.ins().func_addr(types::I64, entry_ref);
            let types_gv = self.module.declare_data_in_func(types_id, b.func);
            let types_addr = b.ins().symbol_value(types::I64, types_gv);
            let fns_gv = self.module.declare_data_in_func(fns_id, b.func);
            let fns_addr = b.ins().symbol_value(types::I64, fns_gv);
            let flags = b.ins().iconst(types::I32, i64::from(flags));
            let ret_ty = b.ins().iconst(types::I32, i64::from(ret_ty));
            let types_len = b.ins().iconst(types::I64, table.len() as i64);
            let fns_len = b.ins().iconst(types::I64, n as i64);
            // What a call may write below a probe limit before a probe sees
            // it: an unprobed frame and a probed one's top, each with room
            // for what a call adds (the return address, the saved frame
            // pointer, arguments on the stack).
            let unprobed = i64::from(self.max_unprobed) + i64::from(self.max_top) + 2 * FRAME_EXTRA;
            let unprobed = b.ins().iconst(types::I64, unprobed);
            let start_ref = self.module.declare_func_in_func(start, b.func);
            let call = b.ins().call(
                start_ref,
                &[argc, argv, entry_addr, flags, ret_ty, types_addr, types_len, fns_addr, fns_len, unprobed],
            );
            let code = b.inst_results(call)[0];
            b.ins().return_(&[code]);
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        self.define(
            main,
            ctx,
            "main".to_string(),
            Frame::Outside,
            debug::FnDebug::new(main, "main".to_string()),
        )
    }
}

/// The functions `main` may reach, by name: through direct calls,
/// closures, function values, and the impls a call through a trait may run
/// (the one its `Self` selects, every one for a generic `Self`, and for a
/// capability handle the host's object's and those of the objects the
/// reached code makes with `cap.new`). A program's SIR also holds its
/// tests, which `main` does not run, and their fakes.
fn reachable(prog: &sir::Program) -> std::collections::HashSet<String> {
    use std::collections::{HashMap, HashSet};
    let mut fns: HashMap<&str, &sir::Function> = HashMap::new();
    for f in &prog.functions {
        fns.entry(f.name.as_str()).or_insert(f);
    }
    let mut seen: HashSet<String> = HashSet::new();
    // The object types each capability handle type may hold.
    let mut objects: HashMap<String, HashSet<Type>> = HashMap::new();
    for (h, o) in abi::CAPABILITIES {
        objects.entry(h.to_string()).or_default().insert(Type::Named(o.to_string(), Vec::new()));
    }
    let impls_for = |tr: &str, method: &str, self_ty: Option<&Type>, out: &mut Vec<String>| {
        for imp in prog.impls.iter().filter(|i| i.trait_name == tr) {
            if let Some(t) = self_ty {
                let mut binds = vec![None; imp.params as usize];
                if !func::match_type(&imp.self_ty, t, &mut binds) {
                    continue;
                }
            }
            match imp.methods.iter().find(|(m, _)| m == method) {
                Some((_, sir::ImplTarget::Fn(f))) => out.push(f.clone()),
                Some(_) => {}
                None => out.push(format!("{tr}.{method}")),
            }
        }
    };
    loop {
        let before = (seen.len(), objects.values().map(HashSet::len).sum::<usize>());
        let mut work: Vec<String> = prog.entry.iter().map(|e| e.function.clone()).collect();
        work.extend(seen.iter().cloned());
        let mut done: HashSet<String> = HashSet::new();
        while let Some(name) = work.pop() {
            if !done.insert(name.clone()) {
                continue;
            }
            seen.insert(name.clone());
            let Some(f) = fns.get(name.as_str()).copied() else { continue };
            let mut found = Vec::new();
            let operand_fn = |o: &sir::Operand, out: &mut Vec<String>| {
                if let sir::Operand::Const(sir::Const::Fn(n)) = o {
                    out.push(n.clone());
                }
            };
            for b in &f.blocks {
                for s in &b.stmts {
                    match &s.kind {
                        sir::StatementKind::Call { callee, args, .. } => {
                            for a in args {
                                operand_fn(a, &mut found);
                            }
                            match callee {
                                sir::Callee::Fn(n, _) => found.push(n.clone()),
                                sir::Callee::Trait(n, tys) => {
                                    let (tr, method) = n.rsplit_once('.').unwrap_or((n, ""));
                                    let mut self_ty = tys.first().cloned().unwrap_or(Type::Unit);
                                    loop {
                                        match &self_ty {
                                            Type::Ref(inner, _) => self_ty = (**inner).clone(),
                                            // A lent capability is its handle.
                                            Type::Named(n, a) if n == "Lent" && a.len() == 1 => {
                                                self_ty = a[0].clone()
                                            }
                                            _ => break,
                                        }
                                    }
                                    let handle = match &self_ty {
                                        Type::Named(h, a) if a.is_empty() => objects.get(h).cloned(),
                                        _ => None,
                                    };
                                    match handle {
                                        Some(objs) => {
                                            for o in &objs {
                                                impls_for(tr, method, Some(o), &mut found);
                                            }
                                        }
                                        None if self_ty.has_params() || matches!(self_ty, Type::Dyn(..)) => {
                                            impls_for(tr, method, None, &mut found);
                                        }
                                        None => impls_for(tr, method, Some(&self_ty), &mut found),
                                    }
                                }
                                sir::Callee::Value(o) => operand_fn(o, &mut found),
                                sir::Callee::Intrinsic(n, tys) => {
                                    if n == "cap.new"
                                        && let (Some(t), Some(Type::Named(h, _))) = (tys.first(), tys.get(1))
                                    {
                                        objects.entry(h.clone()).or_default().insert(t.clone());
                                    }
                                }
                            }
                        }
                        sir::StatementKind::Assign(_, r) => match r {
                            sir::Rvalue::Closure(n, _, ops) => {
                                found.push(n.clone());
                                for o in ops {
                                    operand_fn(o, &mut found);
                                }
                            }
                            sir::Rvalue::Use(o) | sir::Rvalue::Unary(_, o) => operand_fn(o, &mut found),
                            sir::Rvalue::Binary(_, a, b) => {
                                operand_fn(a, &mut found);
                                operand_fn(b, &mut found);
                            }
                            sir::Rvalue::Aggregate(_, ops) => {
                                for o in ops {
                                    operand_fn(o, &mut found);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            work.extend(found);
        }
        let after = (seen.len(), objects.values().map(HashSet::len).sum::<usize>());
        if after == before {
            break;
        }
    }
    // Drop glue runs wherever a value of its type is dropped.
    for f in &prog.functions {
        if f.kind == sir::FnKind::Glue {
            seen.insert(f.name.clone());
        }
    }
    seen
}

/// The capability handle types the program makes objects of, besides the
/// host's: every `cap.new[T, H]` makes an `H` on another object (in the
/// functions `main` may reach); a test clock becomes a `Clock` that way.
fn foreign_objects(
    prog: &sir::Program,
    reached: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for f in prog.functions.iter().filter(|f| reached.contains(&f.name)) {
        for b in &f.blocks {
            for s in &b.stmts {
                if let sir::StatementKind::Call { callee: sir::Callee::Intrinsic(name, tys), .. } = &s.kind
                    && name == "cap.new"
                {
                    match tys.get(1) {
                        Some(Type::Named(h, _)) => {
                            out.insert(h.clone());
                        }
                        _ => {
                            // A handle type the call does not say: any.
                            for (h, _) in abi::CAPABILITIES {
                                out.insert(h.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// A symbol-safe spelling of a SIR name.
fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}
