//! LIR translated one to one into Cranelift IR (D-087, step 1;
//! docs/architecture.md, "LIR"): each operation of a function is the call
//! of Cranelift's function builder that `sisu-codegen` makes where stage 2
//! wrote it, in the same order, so that the functions, the object file and
//! the executable are the ones the program's SIR gives, byte for byte. The
//! module's declarations and data are made in the order the file gives
//! them, the functions compiled on threads and placed in that order
//! (decision 219), and the debug information written from what each
//! function's locals say and from the compiled code (decision 227).

use crate::lir::{self, Bin, Conv, Imm, Item, Load, Op, Ovf, Sig, Un};
use crate::{Checks, Compiled, Error, FRAME_EXTRA, Frame, debug, workers};
use cranelift_codegen::ir::{self, AbiParam, Block, FuncRef, GlobalValue, InstBuilder, MemFlags, Value, types};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, ir::Signature};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch, Variable};
use cranelift_module::{DataDescription, DataId, FuncId, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

/// Compiles a LIR program's text to an object file defining the C `main`,
/// with debug information saying where the sources are.
pub fn compile_lir(text: &str, sources: &debug::Sources) -> Result<Compiled, Error> {
    let lir = lir::read(text)?;
    let mut tr = Tr::new(lir.release)?;
    // The functions before the first that needs the probe reserve (C's
    // `main`) are compiled on threads; that one is written once they are
    // placed, as `sisu-codegen` writes it (decision 234).
    let split = lir
        .items
        .iter()
        .position(|i| matches!(i, Item::Function(f) if f.reserve))
        .unwrap_or(lir.items.len());
    let (first, rest) = lir.items.split_at(split);
    let threads = workers::count();
    if threads > 1 {
        let workers::Channels { jobs, queue, results, done, spare, spares } = workers::Channels::new();
        let senders: Vec<_> = (1..threads).map(|_| results.clone()).collect();
        let isa = tr.isa.clone();
        let stop = std::sync::atomic::AtomicBool::new(false);
        tr.jobs = Some(jobs);
        tr.spares = Some(spares);
        std::thread::scope(|s| {
            let handles: Vec<_> = senders
                .into_iter()
                .filter_map(|r| {
                    let (isa, queue, stop, spare) = (&isa, &queue, &stop, spare.clone());
                    std::thread::Builder::new()
                        .spawn_scoped(s, move || workers::work(&**isa, queue, &r, &spare, stop))
                        .ok()
                })
                .collect();
            let generated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tr.items(first)));
            tr.jobs = None;
            tr.spares = None;
            if matches!(generated, Ok(Ok(()))) {
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
            generated.unwrap_or_else(|e| std::panic::resume_unwind(e))?;
            tr.place(done.try_iter().collect())
        })?;
    } else {
        tr.items(first)?;
    }
    tr.items(rest)?;
    let stats = lir.stats;
    let mut product = tr.module.finish();
    debug::write(&mut product, &lir.sources, &tr.debug, debug::Describe::Lir(&lir.types), sources, &*tr.isa)?;
    let object = product.emit().map_err(|e| Error::internal(format!("writing the object file: {e}")))?;
    Ok(Compiled {
        object,
        functions: stats.functions,
        statements: stats.statements,
        checks: Checks {
            index: stats.index,
            index_proven: stats.index_proven,
            overflow: stats.overflow,
            overflow_proven: stats.overflow_proven,
            tests_decided: stats.tests_decided,
        },
    })
}

/// Whether a LIR program is for the release profile, which `sisu build`
/// reports with its checks.
pub fn lir_is_release(text: &str) -> bool {
    text.lines().take(4).any(|l| l.trim() == "profile release")
}

/// What the translation keeps across a program's functions.
struct Tr {
    module: ObjectModule,
    isa: cranelift_codegen::isa::OwnedTargetIsa,
    call_conv: CallConv,
    /// The declared functions by symbol, and their signatures.
    funcs: HashMap<String, (FuncId, Signature)>,
    /// `dN`, in order.
    data: Vec<DataId>,
    fbctx: FunctionBuilderContext,
    jobs: Option<std::sync::mpsc::Sender<workers::Job>>,
    spares: Option<std::sync::mpsc::Receiver<ir::Function>>,
    handed: Vec<(FuncId, String, Frame, debug::FnDebug)>,
    debug: Vec<debug::FnDebug>,
    max_top: u32,
    max_unprobed: u32,
}

impl Tr {
    fn new(release: bool) -> Result<Tr, Error> {
        let mut flags = settings::builder();
        let set = |f: &mut settings::Builder, k: &str, v: &str| {
            f.set(k, v).map_err(|e| Error::internal(format!("the setting `{k}`: {e}")))
        };
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
        Ok(Tr {
            module: ObjectModule::new(builder),
            isa: shared,
            call_conv,
            funcs: HashMap::new(),
            data: Vec::new(),
            fbctx: FunctionBuilderContext::new(),
            jobs: None,
            spares: None,
            handed: Vec::new(),
            debug: Vec::new(),
            max_top: 0,
            max_unprobed: 0,
        })
    }

    fn items(&mut self, items: &[Item]) -> Result<(), Error> {
        for item in items {
            self.item(item)?;
        }
        Ok(())
    }

    fn item(&mut self, item: &Item) -> Result<(), Error> {
        match item {
            Item::Import(name) => {
                let id = self
                    .module
                    .declare_data(name, cranelift_module::Linkage::Import, true, false)
                    .map_err(|e| Error::internal(e.to_string()))?;
                self.data.push(id);
            }
            Item::Data { bytes, align, funcs } => {
                let id = self
                    .module
                    .declare_anonymous_data(false, false)
                    .map_err(|e| Error::internal(e.to_string()))?;
                let mut d = DataDescription::new();
                d.define(bytes.clone().into_boxed_slice());
                if let Some(a) = align {
                    d.set_align(*a);
                }
                for (off, symbol) in funcs {
                    let f = self.func(symbol)?;
                    let fref = self.module.declare_func_in_data(f, &mut d);
                    d.write_function_addr(*off, fref);
                }
                self.module.define_data(id, &d).map_err(|e| Error::internal(e.to_string()))?;
                self.data.push(id);
            }
            Item::Declare { symbol, linkage, sig } => {
                let sig = self.signature(sig);
                let id = self
                    .module
                    .declare_function(symbol, *linkage, &sig)
                    .map_err(|e| Error::internal(e.to_string()))?;
                self.funcs.insert(symbol.clone(), (id, sig));
            }
            Item::Function(f) => self.function(f)?,
        }
        Ok(())
    }

    fn signature(&self, sig: &Sig) -> Signature {
        match sig {
            Sig::Sisu(n) => crate::sisu_signature(self.call_conv, *n),
            Sig::Plain(params, returns) => {
                let mut s = Signature::new(self.call_conv);
                s.params.extend(params.iter().map(|t| AbiParam::new(*t)));
                s.returns.extend(returns.iter().map(|t| AbiParam::new(*t)));
                s
            }
        }
    }

    fn func(&self, symbol: &str) -> Result<FuncId, Error> {
        self.funcs
            .get(symbol)
            .map(|(id, _)| *id)
            .ok_or_else(|| Error::internal(format!("LIR: `{symbol}` is not declared")))
    }

    /// A context to write a function's IR in, in a function the threads
    /// compiled and gave back when there is one.
    fn context(&mut self) -> Context {
        let mut ctx = match self.spares.as_ref().and_then(|r| r.try_recv().ok()) {
            Some(func) => Context::for_function(func),
            None => Context::new(),
        };
        ctx.func.signature.call_conv = self.call_conv;
        ctx
    }

    /// The probe reserve of a task's stack (decision 234), from the frames
    /// compiled so far.
    fn reserve(&self) -> i64 {
        i64::from(self.max_unprobed) + i64::from(self.max_top) + 2 * FRAME_EXTRA
    }

    fn function(&mut self, f: &lir::Func) -> Result<(), Error> {
        let (id, sig) = self
            .funcs
            .get(&f.symbol)
            .cloned()
            .ok_or_else(|| Error::internal(format!("LIR: `{}` is not declared", f.symbol)))?;
        let mut ctx = self.context();
        ctx.func.signature = sig;
        let mut fbctx = std::mem::take(&mut self.fbctx);
        let result = {
            let b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            self.body(b, f)
        };
        self.fbctx = if result.is_ok() { fbctx } else { FunctionBuilderContext::new() };
        let slots = result.map_err(|e| Error::internal(format!("LIR: `{}`: {e}", f.symbol)))?;
        let mut dbg = debug::FnDebug::new(id, f.name.clone());
        dbg.source = f.source;
        dbg.line = f.line;
        for l in &f.locals {
            let slot = *slots
                .get(l.slot as usize)
                .ok_or_else(|| Error::internal(format!("LIR: `{}`: no slot s{}", f.symbol, l.slot)))?;
            dbg.vars.push(debug::Var {
                name: l.name.clone(),
                ty: debug::DebugTy::Lir(l.ty),
                slot,
                indirect: l.indirect,
                param: l.param,
                line: l.line,
            });
        }
        let frame = match f.frame {
            lir::FrameKind::Probed => Frame::Probed,
            lir::FrameKind::Unprobed => Frame::Unprobed,
            lir::FrameKind::Outside => Frame::Outside,
        };
        self.define(id, ctx, format!("Cranelift rejected `{}`", f.symbol), frame, dbg)
    }

    /// Replays a function's operations through the builder; the slots it
    /// made, for the debug information.
    fn body(&mut self, mut b: FunctionBuilder<'_>, f: &lir::Func) -> Result<Vec<ir::StackSlot>, String> {
        let slots: Vec<ir::StackSlot> = f
            .slots
            .iter()
            .map(|s| b.create_sized_stack_slot(ir::StackSlotData::new(ir::StackSlotKind::ExplicitSlot, *s, 3)))
            .collect();
        let vars: Vec<Variable> = (0..f.vars).map(|_| b.declare_var(types::I64)).collect();
        let mut blocks: Vec<Block> = Vec::new();
        let mut vals: Vec<Value> = Vec::new();
        let mut frefs: HashMap<FuncId, FuncRef> = HashMap::new();
        let mut gvs: HashMap<DataId, GlobalValue> = HashMap::new();
        let flags = MemFlags::trusted();
        // Block `k` and every block before it, made as the lowering made
        // them (decision 273).
        let ensure = |b: &mut FunctionBuilder<'_>, blocks: &mut Vec<Block>, k: u32| -> Block {
            while blocks.len() <= k as usize {
                blocks.push(b.create_block());
            }
            blocks[k as usize]
        };
        let val = |vals: &[Value], v: u32| -> Result<Value, String> {
            vals.get(v as usize).copied().ok_or_else(|| format!("no value v{v}"))
        };
        let many = |vals: &[Value], vs: &[u32]| -> Result<Vec<Value>, String> {
            vs.iter().map(|v| val(vals, *v)).collect()
        };
        let block_args = |vals: &[Value], vs: &[u32]| -> Result<Vec<ir::BlockArg>, String> {
            vs.iter().map(|v| val(vals, *v).map(ir::BlockArg::from)).collect()
        };
        let mut entered = false;
        for op in &f.ops {
            match op {
                Op::Block(k, cold) => {
                    let blk = ensure(&mut b, &mut blocks, *k);
                    if !entered {
                        b.append_block_params_for_function_params(blk);
                    }
                    if *cold {
                        b.set_cold_block(blk);
                    }
                    b.switch_to_block(blk);
                    if !entered {
                        vals.extend_from_slice(b.block_params(blk));
                        entered = true;
                    }
                }
                Op::Loc(l, c) => b.set_srcloc(crate::func::srcloc(*l, *c)),
                Op::Iconst(t, n) => vals.push(b.ins().iconst(*t, *n)),
                Op::Binary(k, x, y) => {
                    let (x, y) = (val(&vals, *x)?, val(&vals, *y)?);
                    let r = match k {
                        Bin::Iadd => b.ins().iadd(x, y),
                        Bin::Isub => b.ins().isub(x, y),
                        Bin::Imul => b.ins().imul(x, y),
                        Bin::Sdiv => b.ins().sdiv(x, y),
                        Bin::Srem => b.ins().srem(x, y),
                        Bin::Ishl => b.ins().ishl(x, y),
                        Bin::Sshr => b.ins().sshr(x, y),
                        Bin::Band => b.ins().band(x, y),
                        Bin::Bor => b.ins().bor(x, y),
                        Bin::Bxor => b.ins().bxor(x, y),
                        Bin::BandNot => b.ins().band_not(x, y),
                        Bin::Fadd => b.ins().fadd(x, y),
                        Bin::Fsub => b.ins().fsub(x, y),
                        Bin::Fmul => b.ins().fmul(x, y),
                        Bin::Fdiv => b.ins().fdiv(x, y),
                    };
                    vals.push(r);
                }
                Op::BinaryImm(k, x, n) => {
                    let x = val(&vals, *x)?;
                    let r = match k {
                        Imm::IaddImm => b.ins().iadd_imm(x, *n),
                        Imm::ImulImm => b.ins().imul_imm(x, *n),
                        Imm::BxorImm => b.ins().bxor_imm(x, *n),
                        Imm::BorImm => b.ins().bor_imm(x, *n),
                    };
                    vals.push(r);
                }
                Op::Unary(k, x) => {
                    let x = val(&vals, *x)?;
                    let r = match k {
                        Un::Ineg => b.ins().ineg(x),
                        Un::Bnot => b.ins().bnot(x),
                        Un::Fneg => b.ins().fneg(x),
                    };
                    vals.push(r);
                }
                Op::Icmp(cc, x, y) => {
                    let (x, y) = (val(&vals, *x)?, val(&vals, *y)?);
                    vals.push(b.ins().icmp(*cc, x, y));
                }
                Op::IcmpImm(cc, x, n) => {
                    let x = val(&vals, *x)?;
                    vals.push(b.ins().icmp_imm(*cc, x, *n));
                }
                Op::Fcmp(cc, x, y) => {
                    let (x, y) = (val(&vals, *x)?, val(&vals, *y)?);
                    vals.push(b.ins().fcmp(*cc, x, y));
                }
                Op::Overflow(k, x, y) => {
                    let (x, y) = (val(&vals, *x)?, val(&vals, *y)?);
                    let (r, of) = match k {
                        Ovf::Sadd => b.ins().sadd_overflow(x, y),
                        Ovf::Ssub => b.ins().ssub_overflow(x, y),
                        Ovf::Smul => b.ins().smul_overflow(x, y),
                    };
                    vals.push(r);
                    vals.push(of);
                }
                Op::Select(c, x, y) => {
                    let (c, x, y) = (val(&vals, *c)?, val(&vals, *x)?, val(&vals, *y)?);
                    vals.push(b.ins().select(c, x, y));
                }
                Op::Convert(k, t, x) => {
                    let x = val(&vals, *x)?;
                    let r = match k {
                        Conv::Uextend => b.ins().uextend(*t, x),
                        Conv::Ireduce => b.ins().ireduce(*t, x),
                        Conv::Bitcast => b.ins().bitcast(*t, MemFlags::new(), x),
                        Conv::Fpromote => b.ins().fpromote(*t, x),
                        Conv::Fdemote => b.ins().fdemote(*t, x),
                    };
                    vals.push(r);
                }
                Op::Load(k, a, off) => {
                    let a = val(&vals, *a)?;
                    let r = match k {
                        Load::Word(t) => b.ins().load(*t, flags, a, *off),
                        Load::U8 => b.ins().uload8(types::I64, flags, a, *off),
                        Load::S8 => b.ins().sload8(types::I64, flags, a, *off),
                    };
                    vals.push(r);
                }
                Op::Store(v, a, off) => {
                    let (v, a) = (val(&vals, *v)?, val(&vals, *a)?);
                    b.ins().store(flags, v, a, *off);
                }
                Op::StackAddr(s) => {
                    let ss = *slots.get(*s as usize).ok_or_else(|| format!("no slot s{s}"))?;
                    vals.push(b.ins().stack_addr(types::I64, ss, 0));
                }
                Op::StackLoad(s) => {
                    let ss = *slots.get(*s as usize).ok_or_else(|| format!("no slot s{s}"))?;
                    vals.push(b.ins().stack_load(types::I64, ss, 0));
                }
                Op::StackStore(v, s) => {
                    let ss = *slots.get(*s as usize).ok_or_else(|| format!("no slot s{s}"))?;
                    let v = val(&vals, *v)?;
                    b.ins().stack_store(v, ss, 0);
                }
                Op::SymbolValue(d) => {
                    let id = *self.data.get(*d as usize).ok_or_else(|| format!("no data d{d}"))?;
                    let gv = *gvs.entry(id).or_insert_with(|| self.module.declare_data_in_func(id, b.func));
                    vals.push(b.ins().symbol_value(types::I64, gv));
                }
                Op::FuncAddr(symbol) => {
                    let id = self.func(symbol).map_err(|e| format!("{e:?}"))?;
                    let r = *frefs.entry(id).or_insert_with(|| self.module.declare_func_in_func(id, b.func));
                    vals.push(b.ins().func_addr(types::I64, r));
                }
                Op::GetStackPointer => vals.push(b.ins().get_stack_pointer(types::I64)),
                Op::Call(symbol, args, n) => {
                    let id = self.func(symbol).map_err(|e| format!("{e:?}"))?;
                    let r = *frefs.entry(id).or_insert_with(|| self.module.declare_func_in_func(id, b.func));
                    let args = many(&vals, args)?;
                    let call = b.ins().call(r, &args);
                    let results = b.inst_results(call);
                    if results.len() != *n as usize {
                        return Err(format!("`{symbol}` gives {} values, not {n}", results.len()));
                    }
                    vals.extend_from_slice(results);
                }
                Op::CallIndirect(params, code, args, n) => {
                    let sig = crate::sisu_signature(self.call_conv, *params);
                    let sr = b.import_signature(sig);
                    let code = val(&vals, *code)?;
                    let args = many(&vals, args)?;
                    let call = b.ins().call_indirect(sr, code, &args);
                    let results = b.inst_results(call);
                    if results.len() != *n as usize {
                        return Err(format!("an indirect call gives {} values, not {n}", results.len()));
                    }
                    vals.extend_from_slice(results);
                }
                Op::Memcpy(d, s, n) => {
                    let (d, s, n) = (val(&vals, *d)?, val(&vals, *s)?, val(&vals, *n)?);
                    let config = self.module.target_config();
                    b.call_memcpy(config, d, s, n);
                }
                Op::Use(x) => {
                    let var = *vars.get(*x as usize).ok_or_else(|| format!("no variable x{x}"))?;
                    vals.push(b.use_var(var));
                }
                Op::Def(x, v) => {
                    let var = *vars.get(*x as usize).ok_or_else(|| format!("no variable x{x}"))?;
                    let v = val(&vals, *v)?;
                    b.def_var(var, v);
                }
                Op::Param(k) => {
                    let blk = ensure(&mut b, &mut blocks, *k);
                    vals.push(b.append_block_param(blk, types::I64));
                }
                Op::ProbeReserve => vals.push(b.ins().iconst(types::I64, self.reserve())),
                Op::Jump(k, args) => {
                    let blk = ensure(&mut b, &mut blocks, *k);
                    let args = block_args(&vals, args)?;
                    b.ins().jump(blk, &args);
                }
                Op::Brif(c, x, xargs, y, yargs) => {
                    let c = val(&vals, *c)?;
                    let bx = ensure(&mut b, &mut blocks, *x);
                    let by = ensure(&mut b, &mut blocks, *y);
                    let (xargs, yargs) = (block_args(&vals, xargs)?, block_args(&vals, yargs)?);
                    b.ins().brif(c, bx, &xargs, by, &yargs);
                }
                Op::Switch(v, default, cases, made) => {
                    // The blocks the lowering had made come before those
                    // the switch makes.
                    if *made > 0 {
                        ensure(&mut b, &mut blocks, made - 1);
                    }
                    let v = val(&vals, *v)?;
                    let mut sw = Switch::new();
                    for (k, blk) in cases {
                        let blk = ensure(&mut b, &mut blocks, *blk);
                        sw.set_entry(u128::from(*k), blk);
                    }
                    let d = ensure(&mut b, &mut blocks, *default);
                    sw.emit(&mut b, v, d);
                }
                Op::Return(vs) => {
                    let vs = many(&vals, vs)?;
                    b.ins().return_(&vs);
                }
                Op::Trap(n) => {
                    b.ins().trap(ir::TrapCode::unwrap_user(*n));
                }
            }
        }
        b.seal_all_blocks();
        b.finalize();
        Ok(slots)
    }

    /// Defines function `id` with the body in `ctx`: compiled here, or
    /// handed to the threads that compile, whose code [`Tr::place`]
    /// defines (as `sisu-codegen` defines its functions, decision 219).
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
    /// written.
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

    fn count_frame(&mut self, frame: Frame, size: u32, top: u32) {
        match frame {
            Frame::Probed => self.max_top = self.max_top.max(top),
            Frame::Unprobed => self.max_unprobed = self.max_unprobed.max(size),
            Frame::Outside => {}
        }
    }
}
