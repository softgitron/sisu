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

mod debug;
mod func;
mod fx;
mod layout;
mod workers;

pub use debug::Sources;

use cranelift_codegen::ir::MemFlags;
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, types};
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
    let mut cg = Codegen::new(prog, env_words, options.release)?;
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
    let missing = std::mem::take(&mut cg.missing);
    let mut product = cg.module.finish();
    step("the code placed", &mut tt);
    debug::write(&mut product, prog, &cg.debug, &mut cg.types, &options.sources, &*cg.isa)?;
    step("the debug information written", &mut tt);
    let object = product.emit().map_err(|e| Error::internal(format!("writing the object file: {e}")))?;
    step("the object emitted", &mut tt);
    Ok(Ok((Compiled { object, functions, statements }, missing)))
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
pub fn link(object: &[u8], runtime: &Path, output: &Path) -> Result<(), Error> {
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
    let out = cmd.arg("-o").arg(output).arg(&obj).arg(runtime).args(native_libs()).output();
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
    pub depth: DataId,
    /// The scheduler's budget, the running task's stack limits, and the
    /// site of the last call that may wait (`sisu_rt::abi::sym`).
    pub budget: DataId,
    pub stack: DataId,
    pub site: DataId,
    /// The largest frame of a compiled function, in bytes: the probe
    /// reserve of a task's stack holds it (decision 203).
    max_frame: u32,
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
    /// The target, which the threads that compile share (decision 219).
    isa: cranelift_codegen::isa::OwnedTargetIsa,
    /// Where the functions go to be compiled on those threads; none when
    /// each is compiled as it is made.
    jobs: Option<std::sync::mpsc::Sender<workers::Job>>,
    /// The functions the threads compiled, cleared, to write the next
    /// ones into.
    spares: Option<std::sync::mpsc::Receiver<ir::Function>>,
    /// The functions handed to the threads, in the order they were made:
    /// each one's id, what an error in it is called, whether its frame
    /// counts towards `max_frame`, and what the debug information says of
    /// it.
    handed: Vec<(FuncId, String, bool, debug::FnDebug)>,
    /// What the debug information says of the functions defined, in the
    /// order they were made (decision 227).
    debug: Vec<debug::FnDebug>,
    /// Surveying (`survey`): a function with a construct the backend does
    /// not compile is recorded in `missing` and given a body that stops.
    survey: bool,
    missing: Vec<(String, String)>,
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
    /// The words of environment the largest `own` closure compiled needs,
    /// beyond what a function value holds: the program is compiled again.
    pub env_needed: u32,
}

/// What a call adds to the callee's frame beyond what Cranelift reports
/// as its size: the return address, the saved frame pointer, and the
/// arguments passed on the stack.
const FRAME_EXTRA: i64 = 256;

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
    fn new(prog: &'p sir::Program, env_words: u32, release: bool) -> Result<Codegen<'p>, Error> {
        let mut flags = settings::builder();
        let set = |f: &mut settings::Builder, k: &str, v: &str| {
            f.set(k, v).map_err(|e| Error::internal(format!("the setting `{k}`: {e}")))
        };
        // The release profile is Cranelift's optimisations (decision 232).
        set(&mut flags, "opt_level", if release { "speed" } else { "none" })?;
        set(&mut flags, "is_pic", "true")?;
        set(&mut flags, "enable_verifier", if cfg!(debug_assertions) { "true" } else { "false" })?;
        let isa = cranelift_native::builder()
            .map_err(|e| Error::internal(format!("this machine: {e}")))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| Error::internal(format!("this machine: {e}")))?;
        let call_conv = isa.default_call_conv();
        let shared = isa.clone();
        let builder = ObjectBuilder::new(isa, "sisu_program", cranelift_module::default_libcall_names())
            .map_err(|e| Error::internal(format!("the object file: {e}")))?;
        let mut module = ObjectModule::new(builder);
        let mut import = |name: &str| {
            module
                .declare_data(name, Linkage::Import, true, false)
                .map_err(|e| Error::internal(e.to_string()))
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
            depth,
            budget,
            stack,
            site,
            max_frame: 0,
            texts: fx::FxMap::default(),
            descs: fx::FxMap::default(),
            glue_ids: Vec::new(),
            thunks: fx::FxMap::default(),
            foreign: foreign_objects(prog, &reachable(prog)),
            fbctx: FunctionBuilderContext::new(),
            isa: shared,
            jobs: None,
            spares: None,
            handed: Vec::new(),
            debug: Vec::new(),
            survey: false,
            missing: Vec::new(),
            carried: fx::FxMap::default(),
            dispatchers: Vec::new(),
            dispatcher_index: fx::FxMap::default(),
            dispatched: false,
            names: 0,
            compiled: 0,
            statements: 0,
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
        enum Call {
            Fn { callee: FuncId, by_value: Option<u32> },
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
                    Call::Fn { callee: self.instance(f, targs.clone())?, by_value }
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
        let mut ctx = self.context();
        ctx.func.signature = sisu_signature(self.call_conv, n as u32);
        let mut fbctx = std::mem::take(&mut self.fbctx);
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
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
                    Call::Fn { callee, by_value } => {
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
                        let r = self.module.declare_func_in_func(*callee, b.func);
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
                        let gv = self.module.declare_data_in_func(*desc, b.func);
                        let d = b.ins().symbol_value(types::I64, gv);
                        let f = intrinsic_fn.ok_or_else(|| Error::internal("no intrinsic entry point"))?;
                        let r = self.module.declare_func_in_func(f, b.func);
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
        self.define(id, ctx, what, true, dbg)
    }

    /// Defines function `id` with the body in `ctx`: compiled here, or
    /// handed to the threads that compile (decision 219), whose code
    /// [`Codegen::place`] defines. `what` names the function in an error;
    /// `frame`: whether its frame counts towards `max_frame` (a task may
    /// run it).
    fn define(
        &mut self,
        id: FuncId,
        mut ctx: Context,
        what: String,
        frame: bool,
        dbg: debug::FnDebug,
    ) -> Result<(), Error> {
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
            if frame {
                self.max_frame = self.max_frame.max(code.frame_size);
            }
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
            if frame {
                self.max_frame = self.max_frame.max(code.frame_size);
            }
            self.debug.push(dbg.compiled(code.bytes.len() as u32, code.debug));
        }
        Ok(())
    }

    /// Everything but `main`: the instances the entry reaches, the
    /// dispatchers and the drop glue; the entry's id, `main`'s flags and
    /// the number of the entry's result type.
    fn generate(&mut self, entry: &sir::Entry) -> Result<(FuncId, u32, u32), Error> {
        let main_id = self.instance(&entry.function, Vec::new())?;
        self.complete()?;
        let ret = self.function(&entry.function).map(|f| f.ret().clone()).unwrap_or(Type::Unit);
        let ret_ty = self.types.id(&self.concrete(&ret, &[])?)?;
        self.complete()?;
        let mut flags = if entry.returns_result { abi::ENTRY_RESULT } else { 0 };
        if entry.takes_system {
            flags |= abi::ENTRY_SYSTEM;
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
        self.define(id, ctx, format!("Cranelift rejected `{name}`"), true, dbg)?;
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
            true,
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
        self.define(id, ctx, name.to_string(), false, debug::FnDebug::new(id, name.to_string()))
    }

    /// A runtime entry point, declared once with its signature.
    pub(crate) fn runtime(&mut self, name: &'static str) -> Result<FuncId, Error> {
        if let Some(&id) = self.runtime.get(name) {
            return Ok(id);
        }
        let (i64_, i32_, i8_, f64_) = (types::I64, types::I32, types::I8, types::F64);
        let (params, returns): (&[ir::Type], &[ir::Type]) = match name {
            sym::START => (&[i32_, i64_, i64_, i32_, i32_, i64_, i64_, i64_, i64_, i64_], &[i32_]),
            sym::YIELD => (&[], &[]),
            sym::ARENA_SLOT => (&[i64_, i64_, i32_, i64_], &[i8_]),
            sym::SHARED_TOUCH => (&[i64_], &[]),
            sym::FRAME => (&[i64_, i64_, i64_, i64_, i64_, i64_], &[]),
            sym::ABORT_UNWIND => (&[], &[]),
            sym::PANIC_OVERFLOW => (&[i32_, i64_, i64_], &[i8_]),
            sym::PANIC_INDEX => (&[i64_, i64_, i64_], &[i8_]),
            sym::PANIC_STACK | sym::PANIC_UNREACHABLE => (&[], &[i8_]),
            sym::INT_OP => (&[i32_, i64_, i64_, i64_], &[i8_]),
            sym::FLOAT_OP => (&[i32_, f64_, f64_], &[f64_]),
            sym::ALLOC => (&[i64_], &[i64_]),
            sym::TASK_ENV => (&[i64_, i32_], &[i64_]),
            sym::FREE | sym::TRACE_RELEASE => (&[i64_], &[]),
            sym::DROP => (&[i64_, i32_], &[i8_]),
            sym::COPY => (&[i64_, i64_, i32_], &[]),
            sym::BINARY => (&[i32_, i64_, i64_, i32_, i64_, i32_], &[i8_]),
            sym::INTRINSIC => (&[i64_, i64_, i64_], &[i8_]),
            sym::SIZED => (&[i32_, i32_, i32_, i64_, i64_, i64_], &[i8_]),
            sym::WIDE => (&[i32_, i32_, i64_, i32_, i64_, i32_, i64_], &[i8_]),
            sym::TABLE => (&[i32_, i64_, i64_, i64_], &[i8_]),
            sym::DICT_SLOT => (&[i64_, i64_, i32_, i32_, i64_], &[i8_]),
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
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
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
            let start_ref = self.module.declare_func_in_func(start, b.func);
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
            // The largest frame, and room for what a call adds to it (the
            // return address, the saved frame pointer, arguments on the
            // stack).
            let max_frame = b.ins().iconst(types::I64, i64::from(self.max_frame) + FRAME_EXTRA);
            let call = b.ins().call(
                start_ref,
                &[argc, argv, entry_addr, flags, ret_ty, types_addr, types_len, fns_addr, fns_len, max_frame],
            );
            let code = b.inst_results(call)[0];
            b.ins().return_(&[code]);
            b.seal_all_blocks();
            b.finalize();
        }
        self.fbctx = fbctx;
        self.define(main, ctx, "main".to_string(), false, debug::FnDebug::new(main, "main".to_string()))
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
