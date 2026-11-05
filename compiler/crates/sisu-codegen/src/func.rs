//! One SIR function instance to Cranelift IR.
//!
//! Every local has a stack slot, except the result and the parameters,
//! whose memory the caller provides; a place is an address computed from
//! its local by its projections; an operand is the words at an address.
//! In the release profile, a local of one word that is a number, a `bool`
//! or a `char` and that no code borrows is a Cranelift variable instead,
//! which lives in a register (decision 246); code that needs its address
//! anyway (a call's result, a key) gets a slot it is copied to and from.
//! Each statement that can panic (a call, a drop, an overflow, an index)
//! is followed by a test of its status, which leads to the failure path of
//! its block: a line of the stack trace, then the block's cleanup, or the
//! return of [`abi::UNWINDING`]; in a cleanup block, the abort of a panic
//! during unwinding.

use crate::fx::FxMap;
use crate::{Codegen, Error, Target};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    self, Block, FuncRef, GlobalValue, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value, types,
};
use cranelift_frontend::{FunctionBuilder, Switch, Variable};
use cranelift_module::{DataId, FuncId, Module};
use sisu_rt::abi::{self, IntOp, Intrinsic, SizedOp, ValueOp, sym};
use sisu_sir as sir;
use sisu_sir::{
    Aggregate, BinOp, Callee, Const, Operand, Place, Proj, Rvalue, StatementKind, Terminator, Type, UnOp,
};

/// An argument of the runtime call a failed check makes: a value, or a
/// constant made in the failure's block.
#[derive(Clone, Copy)]
enum Arg {
    V(Value),
    I32(u32),
    I64(i64),
}

/// Values larger than this many words are copied through memory rather
/// than as words.
const MAX_WORDS: u32 = 16;

/// Translates `f`'s instance for `targs`, and gives the named locals the
/// debug information describes (each one's name, type, stack slot, and
/// whether the slot holds its address rather than the value).
pub(crate) fn translate(
    cg: &mut Codegen<'_>,
    b: FunctionBuilder<'_>,
    f: &sir::Function,
    targs: &[Type],
) -> Result<Vec<crate::debug::Var>, Error> {
    let mut cx = Cx::new(cg, b, f, targs)?;
    cx.body()?;
    let vars = std::mem::take(&mut cx.vars);
    cx.finish()?;
    Ok(vars)
}

/// The source location of the code that follows, as Cranelift carries it
/// to the machine code: the line in the high 20 bits, the column in the
/// low 12 (clamped), for the debug information's line table.
pub(crate) fn srcloc(line: u32, col: u32) -> ir::SourceLoc {
    ir::SourceLoc::new((line.min(0xF_FFFF) << 12) | col.min(0xFFF))
}

enum Storage {
    /// The caller's memory, at this address.
    Param(Value),
    Slot(ir::StackSlot),
    /// A variable, with the slot its value is copied to when code needs
    /// its address, made the first time.
    Var(Variable, Option<ir::StackSlot>),
}

struct Local {
    ty: Type,
    storage: Storage,
}

/// An operand's value: its words, or (when large) the address of a copy.
enum Val {
    Words(Vec<Value>),
    Mem(Value),
}

struct Cx<'a, 'p> {
    cg: &'a mut Codegen<'p>,
    b: FunctionBuilder<'a>,
    f: &'a sir::Function,
    targs: &'a [Type],
    locals: Vec<Local>,
    blocks: Vec<Block>,
    /// The failure paths of each SIR block, made when first needed: one for
    /// each set of texts an interpolation is building where a failure
    /// leaves (decision 381), which the path frees.
    fails: Vec<Vec<(Vec<u32>, Block)>>,
    /// The texts an interpolation is building at the start of each SIR
    /// block ([`building_texts`]), and at the statement being translated.
    building_at: Vec<Vec<u32>>,
    building: Vec<u32>,
    /// The locals each block's cleanup drops ([`cleanup_drops`]), which no
    /// failure path of the block frees again.
    covered: Vec<Vec<u32>>,
    line: Variable,
    col: Variable,
    funcs: FxMap<FuncId, FuncRef>,
    data: FxMap<DataId, GlobalValue>,
    /// The function's name and file for its stack trace lines; `None` for
    /// drop glue and the standard library, which traces leave out.
    trace: Option<(DataId, usize, DataId, usize)>,
    /// The function's name as a trace shows it, for the stack overflow
    /// its entry may report (decision 234).
    name: (DataId, usize),
    /// The function's file, for the site of a call that may wait.
    file: (DataId, usize),
    glue: bool,
    /// The SIR block being translated.
    cur: usize,
    /// The named locals, for the debug information.
    vars: Vec<crate::debug::Var>,
    /// The parameters whose addresses are kept in slots of their own for
    /// the debug information, stored after the probe: nothing below the
    /// top of the frame is written before it (decision 234).
    kept: Vec<(Value, ir::StackSlot)>,
    /// The variables whose slot the statement being translated writes
    /// (a call's result): each is read back from its slot after it.
    reload: Vec<usize>,
    /// The slot of each local whose iterator lives in the frame
    /// ([`frame_iterators`]).
    frames: Vec<Option<ir::StackSlot>>,
    /// In the release profile, what is known about the integers at each
    /// point (decision 270), and the state before the statement being
    /// translated.
    facts: Option<crate::facts::Facts>,
    known: Option<crate::facts::State>,
    /// The failed checks of the SIR block being translated, written after
    /// its terminator, each with the texts being built where it is.
    deferred: Vec<(Block, &'static str, [Arg; 3], Vec<u32>)>,
    /// The blocks every path from which ends in a panic or in unwinding:
    /// cold, placed after the rest of the function.
    cold: Vec<bool>,
}

/// Whether a type is a number, `bool`, `char` or `()`: one word or none,
/// compared and computed by instructions.
fn scalar_kind(t: &Type) -> Option<Scalar> {
    match t {
        Type::Int => Some(Scalar::Int),
        Type::Float => Some(Scalar::Float),
        Type::Bool => Some(Scalar::Bool),
        Type::Char => Some(Scalar::Char),
        Type::Unit => Some(Scalar::Unit),
        Type::Named(n, a) if a.is_empty() => match n.as_str() {
            "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "DateTime" => Some(Scalar::Int),
            "u64" => Some(Scalar::U64),
            "f32" => Some(Scalar::F32),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    Int,
    U64,
    Float,
    F32,
    Bool,
    Char,
    Unit,
}

impl<'a, 'p> Cx<'a, 'p> {
    fn new(
        cg: &'a mut Codegen<'p>,
        mut b: FunctionBuilder<'a>,
        f: &'a sir::Function,
        targs: &'a [Type],
    ) -> Result<Cx<'a, 'p>, Error> {
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params: Vec<Value> = b.block_params(entry).to_vec();
        let mut locals = Vec::new();
        let mut vars = Vec::new();
        let mut kept = Vec::new();
        let closure = matches!(f.kind, sir::FnKind::Closure(role) if role != sir::ClosureRole::Task);
        let tys: Vec<Type> = f.locals.iter().map(|l| cg.concrete(&l.ty, targs)).collect::<Result<_, _>>()?;
        let in_registers =
            if cg.release { register_locals(f, &tys, closure) } else { vec![false; tys.len()] };
        let framed = frame_iterators(f, &tys);
        let facts = cg.release.then(|| {
            let constant = |name: &str| match cg.function(name).and_then(returned_constant) {
                Some(Const::Int(c)) => Some(*c),
                _ => None,
            };
            crate::facts::Facts::of(f, &tys, &in_registers, &framed, &constant)
        });
        let frames = framed
            .into_iter()
            .map(|framed| {
                framed.then(|| {
                    b.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        abi::ITER_FRAME,
                        3,
                    ))
                })
            })
            .collect();
        for (i, (l, ty)) in f.locals.iter().zip(tys).enumerate() {
            let storage = if in_registers[i] {
                let var = b.declare_var(types::I64);
                // A parameter's value is read once; another local starts
                // as zero, which nothing reads before it is assigned.
                let first = match params.get(i) {
                    Some(p) if i <= f.param_count as usize => {
                        b.ins().load(types::I64, MemFlags::trusted(), *p, 0)
                    }
                    _ => b.ins().iconst(types::I64, 0),
                };
                b.def_var(var, first);
                Storage::Var(var, None)
            } else if i == 1 && closure && cg.types.size(&ty)? > cg.types.env_words * abi::WORD {
                // A closure's environment is in the function value, whose
                // words after its code the call passes the address of; one
                // larger than they are is a slot of the frame that made the
                // closure, whose address is the first of them (decision 224).
                Storage::Param(b.ins().load(types::I64, MemFlags::trusted(), params[1], 0))
            } else if i <= f.param_count as usize {
                Storage::Param(params[i])
            } else {
                let size = cg.types.size(&ty)?.max(8);
                Storage::Slot(b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                )))
            };
            // A named local is described by its slot; a parameter's (and the
            // environment's) address is kept in a slot of its own, so that
            // a debugger finds the value through it. A variable has neither.
            if let Some(name) = l.name.as_deref().filter(|_| !drop_flag(l)) {
                let name = name.split('(').next().unwrap_or(name).to_string();
                let (slot, indirect) = match &storage {
                    Storage::Slot(ss) => (*ss, false),
                    Storage::Param(v) => {
                        let ss =
                            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
                        kept.push((*v, ss));
                        (ss, true)
                    }
                    Storage::Var(..) => {
                        locals.push(Local { ty, storage });
                        continue;
                    }
                };
                let param = i >= 1 && i <= f.param_count as usize;
                vars.push(crate::debug::Var {
                    name,
                    ty: crate::debug::DebugTy::Sir(ty.clone()),
                    slot,
                    indirect,
                    param,
                    line: l.loc.map(|l| l.line),
                });
            }
            locals.push(Local { ty, storage });
        }
        let line = b.declare_var(types::I64);
        let col = b.declare_var(types::I64);
        let start = f.loc.map(|l| (i64::from(l.line), i64::from(l.col))).unwrap_or((0, 0));
        let v = b.ins().iconst(types::I64, start.0);
        b.def_var(line, v);
        let v = b.ins().iconst(types::I64, start.1);
        b.def_var(col, v);
        let blocks: Vec<Block> = f.blocks.iter().map(|_| b.create_block()).collect();
        // The runtime in Sisu is the runtime's own code, as drop glue is
        // (decision 336).
        let glue = f.kind == sir::FnKind::Glue || cg.library;
        let file = cg.prog.sources.get(f.source as usize).cloned().unwrap_or_default();
        let std = file.starts_with("<std>/") || file.starts_with("<shims>/");
        let file_text = (cg.text(file.as_bytes())?, file.len());
        let shown = display_fn_name(&f.name, cg.prog);
        let name = (cg.text(shown.as_bytes())?, shown.len());
        let trace = if glue || std { None } else { Some((name.0, name.1, file_text.0, file.len())) };
        let fails = vec![Vec::new(); f.blocks.len()];
        let mut cx = Cx {
            cg,
            b,
            f,
            targs,
            locals,
            blocks,
            fails,
            building_at: building_texts(f),
            building: Vec::new(),
            covered: (0..f.blocks.len()).map(|i| cleanup_drops(f, i)).collect(),
            line,
            col,
            funcs: FxMap::default(),
            data: FxMap::default(),
            trace,
            name,
            file: file_text,
            glue,
            cur: 0,
            vars,
            kept,
            reload: Vec::new(),
            frames,
            facts,
            known: None,
            deferred: Vec::new(),
            cold: cold_blocks(f),
        };
        cx.prologue()?;
        Ok(cx)
    }

    // -------------------------------------------------------------- helpers

    fn func_ref(&mut self, id: FuncId) -> FuncRef {
        if let Some(r) = self.funcs.get(&id) {
            return *r;
        }
        let r = self.cg.module.declare_func_in_func(id, self.b.func);
        self.funcs.insert(id, r);
        r
    }

    fn data_addr(&mut self, id: DataId) -> Value {
        let gv = match self.data.get(&id) {
            Some(gv) => *gv,
            None => {
                let gv = self.cg.module.declare_data_in_func(id, self.b.func);
                self.data.insert(id, gv);
                gv
            }
        };
        // The runtime's words of the running thread (decision 332).
        if [self.cg.depth, self.cg.budget, self.cg.stack, self.cg.site].contains(&id) {
            return self.b.ins().tls_value(types::I64, gv);
        }
        self.b.ins().symbol_value(types::I64, gv)
    }

    fn rt(&mut self, name: &'static str, args: &[Value]) -> Result<Option<Value>, Error> {
        let id = self.cg.runtime(name)?;
        let r = self.func_ref(id);
        let call = self.b.ins().call(r, args);
        Ok(self.b.inst_results(call).first().copied())
    }

    fn rt_status(&mut self, name: &'static str, args: &[Value]) -> Result<Value, Error> {
        self.rt(name, args)?.ok_or_else(|| Error::internal(format!("`{name}` returns nothing")))
    }

    /// A call of the runtime in Sisu (decisions 336 and 350), of Sisu's
    /// convention: the address of a word for its result, then each
    /// argument's, in memory of its own, and a status that leads to the
    /// failure path. The address of the result's word.
    fn rt_sisu(&mut self, name: &'static str, args: &[Value]) -> Result<Value, Error> {
        let ret = self.slot(8);
        let slots: Vec<Value> = args.iter().map(|_| self.slot(8)).collect();
        for (&a, &s) in args.iter().zip(&slots) {
            self.store_word(a, s, 0);
        }
        let mut all = Vec::with_capacity(slots.len() + 1);
        all.push(ret);
        all.extend(slots);
        let st = self.rt_status(name, &all)?;
        self.check(st);
        Ok(ret)
    }

    fn iconst(&mut self, v: i64) -> Value {
        self.b.ins().iconst(types::I64, v)
    }

    fn i32const(&mut self, v: u32) -> Value {
        self.b.ins().iconst(types::I32, i64::from(v))
    }

    fn slot(&mut self, size: u32) -> Value {
        let ss =
            self.b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, size.max(8), 3));
        self.b.ins().stack_addr(types::I64, ss, 0)
    }

    fn local_addr(&mut self, l: sir::LocalId) -> Result<Value, Error> {
        match self.locals.get(l.0 as usize).map(|x| &x.storage) {
            Some(Storage::Param(v)) => Ok(*v),
            Some(Storage::Slot(ss)) => {
                let ss = *ss;
                Ok(self.b.ins().stack_addr(types::I64, ss, 0))
            }
            // A variable's value, copied to its slot.
            Some(Storage::Var(var, slot)) => {
                let (var, slot) = (*var, *slot);
                let ss = match slot {
                    Some(ss) => ss,
                    None => {
                        let ss = self.b.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            8,
                            3,
                        ));
                        if let Some(x) = self.locals.get_mut(l.0 as usize) {
                            x.storage = Storage::Var(var, Some(ss));
                        }
                        ss
                    }
                };
                let v = self.b.use_var(var);
                self.b.ins().stack_store(v, ss, 0);
                Ok(self.b.ins().stack_addr(types::I64, ss, 0))
            }
            None => Err(Error::internal(format!("no local _{}", l.0))),
        }
    }

    /// The variable a local is kept in, if it is one.
    fn var_of(&self, l: sir::LocalId) -> Option<Variable> {
        match self.locals.get(l.0 as usize).map(|x| &x.storage) {
            Some(Storage::Var(v, _)) => Some(*v),
            _ => None,
        }
    }

    /// The variables whose slots the statement wrote, read back.
    fn reload_vars(&mut self) {
        for l in std::mem::take(&mut self.reload) {
            if let Some(Local { storage: Storage::Var(var, Some(ss)), .. }) = self.locals.get(l) {
                let (var, ss) = (*var, *ss);
                let v = self.b.ins().stack_load(types::I64, ss, 0);
                self.b.def_var(var, v);
            }
        }
    }

    fn load(&mut self, addr: Value, off: u32) -> Value {
        self.b.ins().load(types::I64, MemFlags::trusted(), addr, off as i32)
    }

    fn store_word(&mut self, v: Value, addr: Value, off: u32) {
        self.b.ins().store(MemFlags::trusted(), v, addr, off as i32);
    }

    /// The address of what the reference of type `ty` at `addr` refers
    /// to: the address it holds, except that a reference to a `dyn` is the
    /// `dyn`'s own two words, in place.
    fn through(&mut self, addr: Value, ty: &Type) -> Value {
        match ty {
            Type::Ref(inner, _) if crate::layout::fat(inner) => addr,
            _ => self.load(addr, 0),
        }
    }

    fn concrete(&self, t: &Type) -> Result<Type, Error> {
        self.cg.concrete(t, self.targs)
    }

    /// Counts a copy of the value of type `ty` at `addr` when it holds
    /// capability handles (decision 237).
    fn count_at(&mut self, addr: Value, ty: &Type) -> Result<(), Error> {
        if !self.cg.types.counted(ty)? {
            return Ok(());
        }
        // A function value counts only what its environment holds, when
        // its last word names the environment's type (decision 248).
        if let Some(env) = self.fn_env(addr, ty)? {
            let (tw, done) = env;
            let at = self.b.ins().iadd_imm(addr, i64::from(abi::WORD));
            let t = self.b.ins().iadd_imm(tw, -1);
            let t = self.b.ins().ireduce(types::I32, t);
            self.rt(sym::RETAIN, &[at, t])?;
            self.b.ins().jump(done, &[]);
            self.b.switch_to_block(done);
            return Ok(());
        }
        let id = self.type_id(ty)?;
        let idv = self.i32const(id);
        self.rt(sym::RETAIN, &[addr, idv])?;
        Ok(())
    }

    /// For a function value at `addr`: its last word, the type of what its
    /// environment holds plus one (decision 248), with the code from here
    /// on reached only when it is not 0, and the block that follows.
    fn fn_env(&mut self, addr: Value, ty: &Type) -> Result<Option<(Value, ir::Block)>, Error> {
        if !matches!(ty, Type::Fn(..)) {
            return Ok(None);
        }
        let size = self.size(ty)?;
        let tw = self.load(addr, size - abi::WORD);
        let go = self.b.create_block();
        let done = self.b.create_block();
        self.b.ins().brif(tw, go, &[], done, &[]);
        self.b.switch_to_block(go);
        Ok(Some((tw, done)))
    }

    fn size(&mut self, t: &Type) -> Result<u32, Error> {
        self.cg.types.size(t)
    }

    fn type_id(&mut self, t: &Type) -> Result<u32, Error> {
        // The runtime in Sisu has no type table of its own (decision 336).
        if self.cg.library {
            return Err(Error::unsupported(format!(
                "a value the runtime walks by its type (`{}`), in the runtime's own code",
                sir::write::ty(t)
            )));
        }
        self.cg.types.id(t)
    }

    /// The words of the value at `addr`.
    fn read(&mut self, addr: Value, size: u32) -> Val {
        if size / 8 > MAX_WORDS {
            let tmp = self.slot(size);
            self.copy_bytes(tmp, addr, size);
            return Val::Mem(tmp);
        }
        Val::Words((0..size / 8).map(|i| self.load(addr, i * 8)).collect())
    }

    /// Writes a value of `size` bytes at `addr`.
    fn write(&mut self, addr: Value, v: &Val, size: u32) {
        match v {
            Val::Words(ws) => {
                for (i, w) in ws.iter().enumerate().take((size / 8) as usize) {
                    self.store_word(*w, addr, (i * 8) as u32);
                }
            }
            Val::Mem(src) => {
                let src = *src;
                self.copy_bytes(addr, src, size);
            }
        }
    }

    fn copy_bytes(&mut self, dst: Value, src: Value, size: u32) {
        if size / 8 <= MAX_WORDS {
            for i in 0..size / 8 {
                let w = self.load(src, i * 8);
                self.store_word(w, dst, i * 8);
            }
            return;
        }
        let n = self.iconst(i64::from(size));
        let config = self.cg.module.target_config();
        self.b.call_memcpy(config, dst, src, n);
    }

    /// The value in memory: its own address, or a new slot holding it.
    fn in_memory(&mut self, v: Val, size: u32) -> Value {
        match v {
            Val::Mem(a) => a,
            Val::Words(ws) => {
                let a = self.slot(size);
                self.write(a, &Val::Words(ws), size);
                a
            }
        }
    }

    fn first_word(&mut self, v: &Val) -> Result<Value, Error> {
        match v {
            Val::Words(ws) => ws.first().copied().ok_or_else(|| Error::internal("a value without words")),
            Val::Mem(a) => {
                let a = *a;
                Ok(self.load(a, 0))
            }
        }
    }

    fn set_loc(&mut self, loc: sir::Loc) {
        self.b.set_srcloc(srcloc(loc.line, loc.col));
        let l = self.iconst(i64::from(loc.line));
        self.b.def_var(self.line, l);
        let c = self.iconst(i64::from(loc.col));
        self.b.def_var(self.col, c);
    }

    // --------------------------------------------------------- frame depth

    fn depth_addr(&mut self) -> Value {
        let id = self.cg.depth;
        self.data_addr(id)
    }

    /// The entry of the function: the stack probe; then, but for drop
    /// glue, the count of the frame, which panics past the limit of nested
    /// calls (as the interpreter's frame limit does), and the yield point
    /// of a call (decision 85: the interpreter's calls of Sisu functions).
    fn prologue(&mut self) -> Result<(), Error> {
        self.probe();
        // The parameters' addresses, for a debugger, once the probe has
        // passed. The code so far is the prologue's in the debug
        // information, with no line of its own: the declaration's line
        // begins after it, where a debugger's breakpoint on the function
        // stops with the parameters in place.
        for (v, ss) in std::mem::take(&mut self.kept) {
            self.b.ins().stack_store(v, ss, 0);
        }
        if let Some(l) = self.f.loc {
            self.b.set_srcloc(srcloc(l.line, l.col));
        }
        if self.glue {
            let first = self.blocks[0];
            self.b.ins().jump(first, &[]);
            return Ok(());
        }
        let a = self.depth_addr();
        let d = self.load(a, 0);
        let d1 = self.b.ins().iadd_imm(d, 1);
        self.store_word(d1, a, 0);
        let over = self.b.ins().icmp_imm(IntCC::SignedGreaterThan, d1, abi::MAX_DEPTH);
        let overflow = self.b.create_block();
        let refused = self.b.create_block();
        let counted = self.b.create_block();
        self.b.ins().brif(over, overflow, &[], counted, &[]);
        self.b.set_cold_block(overflow);
        self.b.switch_to_block(overflow);
        // Past the limit, the runtime raises the overflow, unless a panic's
        // unwinding has room left beyond it (decision 234).
        let name = self.data_addr(self.name.0);
        let len = self.iconst(self.name.1 as i64);
        let st = self.rt_status(sym::PANIC_STACK, &[name, len])?;
        self.b.ins().brif(st, refused, &[], counted, &[]);
        self.b.set_cold_block(refused);
        self.b.switch_to_block(refused);
        let a = self.depth_addr();
        self.store_word(d, a, 0);
        self.b.ins().return_(&[st]);
        self.b.switch_to_block(counted);
        self.yield_check(true)?;
        let first = self.blocks[0];
        self.b.ins().jump(first, &[]);
        Ok(())
    }

    /// The stack probe (decision 203): the stack pointer, below the frame
    /// the prologue made, against the task's limit. Under it, a stack
    /// pointer at or above the real limit only crossed the decommit
    /// watermark: the task is marked dirty and the limit lowered. One below
    /// it has exhausted the stack: the function marks the failure with its
    /// name, which the panic's message gives (decision 234), lowers the
    /// limit to the unwinding floor, and returns at once, calling nothing
    /// and writing nothing below its frame (the runtime raises the panic
    /// when it next looks at the panics in flight).
    fn probe(&mut self) {
        let sp = self.b.ins().get_stack_pointer(types::I64);
        let stack = self.data_addr(self.cg.stack);
        let limit = self.load(stack, abi::STACK_PROBE);
        let below = self.b.ins().icmp(IntCC::UnsignedLessThan, sp, limit);
        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(below, slow, &[], cont, &[]);
        self.b.set_cold_block(slow);
        self.b.switch_to_block(slow);
        let real = self.load(stack, abi::STACK_REAL);
        let under = self.b.ins().icmp(IntCC::UnsignedLessThan, sp, real);
        let watermark = self.b.create_block();
        let fail = self.b.create_block();
        self.b.ins().brif(under, fail, &[], watermark, &[]);
        self.b.switch_to_block(watermark);
        let one = self.iconst(1);
        self.store_word(one, stack, abi::STACK_DIRTY);
        self.store_word(real, stack, abi::STACK_PROBE);
        self.b.ins().jump(cont, &[]);
        self.b.set_cold_block(fail);
        self.b.switch_to_block(fail);
        let name = self.data_addr(self.name.0);
        self.store_word(name, stack, abi::STACK_FAILED);
        let len = self.iconst(self.name.1 as i64);
        self.store_word(len, stack, abi::STACK_FAILED_LEN);
        let floor = self.load(stack, abi::STACK_FLOOR);
        self.store_word(floor, stack, abi::STACK_PROBE);
        let st = self.b.ins().iconst(types::I8, i64::from(abi::UNWINDING));
        self.b.ins().return_(&[st]);
        self.b.switch_to_block(cont);
    }

    /// A call of a constant's function that only returns a literal, read
    /// without the call (decision 260): what its entry does that anything
    /// can see, the count of nested calls past the limit (the overflow,
    /// named after the constant) and the yield point, then the literal.
    fn constant_read(&mut self, dest: &Place, name: &str, c: &Const) -> Result<(), Error> {
        let a = self.depth_addr();
        let d = self.load(a, 0);
        let d1 = self.b.ins().iadd_imm(d, 1);
        let over = self.b.ins().icmp_imm(IntCC::SignedGreaterThan, d1, abi::MAX_DEPTH);
        let (overflow, counted) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(over, overflow, &[], counted, &[]);
        self.b.set_cold_block(overflow);
        self.b.switch_to_block(overflow);
        // As the function's entry does: the count is one more while the
        // runtime decides, and back as it was after.
        self.store_word(d1, a, 0);
        let shown = display_fn_name(name, self.cg.prog);
        let text = self.cg.text(shown.as_bytes())?;
        let text = self.data_addr(text);
        let len = self.iconst(shown.len() as i64);
        let st = self.rt_status(sym::PANIC_STACK, &[text, len])?;
        let a = self.depth_addr();
        self.store_word(d, a, 0);
        self.check(st);
        self.b.ins().jump(counted, &[]);
        self.b.switch_to_block(counted);
        self.yield_check(false)?;
        self.statement(&StatementKind::Assign(dest.clone(), Rvalue::Use(Operand::Const(c.clone()))))
    }

    /// A yield point (decision 85): one less of the budget, and the
    /// scheduler when it is spent. The scheduler's call fails when a
    /// property's case has run past its budget (decision 540): at a
    /// function's `entry` the failure gives the frame's count back and
    /// returns, elsewhere it takes the failure path of the block.
    fn yield_check(&mut self, entry: bool) -> Result<(), Error> {
        // The runtime's own code passes none (decision 336).
        if self.cg.library {
            return Ok(());
        }
        let a = self.data_addr(self.cg.budget);
        let n = self.load(a, 0);
        let n1 = self.b.ins().iadd_imm(n, -1);
        self.store_word(n1, a, 0);
        let spent = self.b.ins().icmp_imm(IntCC::SignedLessThanOrEqual, n1, 0);
        let slow = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(spent, slow, &[], cont, &[]);
        self.b.set_cold_block(slow);
        self.b.switch_to_block(slow);
        let st = self.rt_status(sym::YIELD, &[])?;
        let fail = if entry { self.b.create_block() } else { self.fail_block() };
        self.b.ins().brif(st, fail, &[], cont, &[]);
        if entry {
            self.b.set_cold_block(fail);
            self.b.switch_to_block(fail);
            self.leave(abi::UNWINDING);
        }
        self.b.switch_to_block(cont);
        Ok(())
    }

    /// Records where the running code is before a call that may wait, for
    /// the deadlock report; and whether the function is `pure`, which a
    /// scope opened here needs (06, "Effects").
    fn record_site(&mut self) {
        let site = self.data_addr(self.cg.site);
        let (file, len) = self.file;
        let file = self.data_addr(file);
        self.store_word(file, site, abi::SITE_FILE);
        let len = self.iconst(len as i64);
        self.store_word(len, site, abi::SITE_FILE_LEN);
        let line = self.b.use_var(self.line);
        self.store_word(line, site, abi::SITE_LINE);
        let col = self.b.use_var(self.col);
        self.store_word(col, site, abi::SITE_COL);
        let flags = self.iconst(if self.f.pure { abi::SITE_PURE as i64 } else { 0 });
        self.store_word(flags, site, abi::SITE_FLAGS);
    }

    fn leave(&mut self, status: u8) {
        if !self.glue {
            let a = self.depth_addr();
            let d = self.load(a, 0);
            let d1 = self.b.ins().iadd_imm(d, -1);
            self.store_word(d1, a, 0);
        }
        let s = self.b.ins().iconst(types::I8, i64::from(status));
        self.b.ins().return_(&[s]);
    }

    // ------------------------------------------------------- failure paths

    fn fail_block(&mut self) -> Block {
        let building = self.building.clone();
        self.fail_block_for(building)
    }

    /// The failure path of the SIR block being translated that frees the
    /// texts `building` holds (decision 381).
    fn fail_block_for(&mut self, building: Vec<u32>) -> Block {
        if let Some((_, b)) = self.fails[self.cur].iter().find(|(set, _)| *set == building) {
            return *b;
        }
        let b = self.b.create_block();
        self.fails[self.cur].push((building, b));
        b
    }

    /// Continues when `status` is [`abi::OK`]; else takes the failure path.
    fn check(&mut self, status: Value) {
        let fail = self.fail_block();
        let cont = self.b.create_block();
        self.b.ins().brif(status, fail, &[], cont, &[]);
        self.b.switch_to_block(cont);
    }

    /// Continues unless `bad` holds; else calls the runtime entry point
    /// that raises the panic (`raise`, with `args`, the constants among
    /// them made there) and takes the failure path. The failure's block is
    /// written after the SIR block's terminator, out of the way of the
    /// code that follows the check, so the common path is one branch not
    /// taken (decision 270).
    fn guard_if(&mut self, bad: Value, raise: &'static str, args: [Arg; 3]) -> Result<(), Error> {
        let cont = self.b.create_block();
        let failed = self.b.create_block();
        self.b.ins().brif(bad, failed, &[], cont, &[]);
        self.deferred.push((failed, raise, args, self.building.clone()));
        self.b.switch_to_block(cont);
        Ok(())
    }

    /// A failed check's block: the runtime raises the panic, then the
    /// failure path of the SIR block.
    fn raise(&mut self, raise: &'static str, args: [Arg; 3], building: Vec<u32>) -> Result<(), Error> {
        let args: Vec<Value> = args
            .iter()
            .map(|a| match a {
                Arg::V(v) => *v,
                Arg::I32(n) => self.i32const(*n),
                Arg::I64(n) => self.iconst(*n),
            })
            .collect();
        self.rt(raise, &args)?;
        let fail = self.fail_block_for(building);
        self.b.ins().jump(fail, &[]);
        Ok(())
    }

    fn emit_fails(&mut self) -> Result<(), Error> {
        for i in 0..self.fails.len() {
            for (building, fb) in std::mem::take(&mut self.fails[i]) {
                self.emit_fail(i, &building, fb)?;
            }
        }
        Ok(())
    }

    /// The failure path `fb` of SIR block `i`: the trace's line, the texts
    /// an interpolation was building freed (decision 381), then the
    /// block's cleanup, or the return of the failure.
    fn emit_fail(&mut self, i: usize, building: &[u32], fb: Block) -> Result<(), Error> {
        self.b.set_cold_block(fb);
        self.b.switch_to_block(fb);
        let block = &self.f.blocks[i];
        if let Some(l) = block.loc.or(self.f.loc) {
            self.b.set_srcloc(srcloc(l.line, l.col));
        }
        if block.cleanup {
            self.rt(sym::ABORT_UNWIND, &[])?;
            self.b.ins().trap(ir::TrapCode::unwrap_user(1));
            return Ok(());
        }
        if let Some((n, nl, fl, fll)) = self.trace {
            let name = self.data_addr(n);
            let file = self.data_addr(fl);
            let nl = self.iconst(nl as i64);
            let fll = self.iconst(fll as i64);
            let line = self.b.use_var(self.line);
            let col = self.b.use_var(self.col);
            self.rt(sym::FRAME, &[name, nl, file, fll, line, col])?;
        }
        for &l in building.iter().rev() {
            self.free_text(l)?;
        }
        match block.unwind {
            Some(u) => {
                let target = self.blocks[u.0 as usize];
                self.b.ins().jump(target, &[]);
            }
            None => self.leave(abi::UNWINDING),
        }
        Ok(())
    }

    /// Frees the `String` local `l` holds, which an interpolation was
    /// building where the function leaves without its statement's end:
    /// nothing of the SIR drops it there (decision 381).
    fn free_text(&mut self, l: u32) -> Result<(), Error> {
        let addr = self.local_addr(sir::LocalId(l))?;
        let ptr = self.load(addr, 0);
        let free = self.b.create_block();
        let done = self.b.create_block();
        self.b.ins().brif(ptr, free, &[], done, &[]);
        self.b.switch_to_block(free);
        self.rt(sym::FREE, &[ptr])?;
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
        Ok(())
    }

    // ---------------------------------------------------------------- body

    fn body(&mut self) -> Result<(), Error> {
        for (i, block) in self.f.blocks.iter().enumerate() {
            self.cur = i;
            let cb = self.blocks[i];
            if self.cold[i] {
                self.b.set_cold_block(cb);
            }
            self.b.switch_to_block(cb);
            self.known = self.facts.as_ref().and_then(|f| f.entry(i));
            self.building = self.building_at[i].clone();
            for s in &block.stmts {
                self.cg.statements += 1;
                if let Some(loc) = s.loc {
                    self.set_loc(loc);
                }
                building_taken(&mut self.building, &s.kind);
                self.statement(&s.kind)?;
                self.reload_vars();
                if let (Some(f), Some(st)) = (&self.facts, &mut self.known) {
                    f.step(st, &s.kind);
                }
                building_begun(&mut self.building, &s.kind, &self.covered[i]);
            }
            self.cg.statements += 1;
            if let Some(loc) = block.loc {
                self.set_loc(loc);
            }
            self.terminator(&block.term)?;
            for (failed, raise, args, building) in std::mem::take(&mut self.deferred) {
                self.b.switch_to_block(failed);
                self.raise(raise, args, building)?;
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<(), Error> {
        self.emit_fails()?;
        self.b.seal_all_blocks();
        self.b.finalize();
        Ok(())
    }

    fn terminator(&mut self, t: &Terminator) -> Result<(), Error> {
        match t {
            Terminator::Goto(bb) => {
                // A jump to a block at or before this one is a loop's back
                // edge: a yield point (decision 85).
                if bb.0 as usize <= self.cur {
                    self.yield_check(false)?;
                }
                let target = self.blocks[bb.0 as usize];
                self.b.ins().jump(target, &[]);
            }
            Terminator::Switch { value, cases, default }
                if let (Some(f), Some(st)) = (&self.facts, &self.known)
                    && let Some(v) = f.decided(st, value) =>
            {
                // A test the facts decide (decision 270): a jump to where
                // its value goes.
                self.cg.checks.tests_decided += 1;
                let k = u64::from(v);
                let t = cases.iter().find(|(c, _)| *c as u64 == k).map_or(default.0, |(_, b)| b.0) as usize;
                if t <= self.cur {
                    self.yield_check(false)?;
                }
                let target = self.blocks[t];
                self.b.ins().jump(target, &[]);
            }
            Terminator::Switch { value, cases, default } => {
                let v = self.scalar(value)?.0;
                // A back edge goes through a block of its own, which yields.
                let mut back: Vec<(usize, Block)> = Vec::new();
                let mut target = |cx: &mut Self, t: usize| -> Block {
                    if t > cx.cur {
                        return cx.blocks[t];
                    }
                    if let Some((_, b)) = back.iter().find(|(u, _)| *u == t) {
                        return *b;
                    }
                    let b = cx.b.create_block();
                    back.push((t, b));
                    b
                };
                let mut sw = Switch::new();
                for (k, bb) in cases {
                    let blk = target(self, bb.0 as usize);
                    sw.set_entry(u128::from(*k as u64), blk);
                }
                let d = target(self, default.0 as usize);
                sw.emit(&mut self.b, v, d);
                for (t, blk) in back {
                    self.b.switch_to_block(blk);
                    self.yield_check(false)?;
                    let to = self.blocks[t];
                    self.b.ins().jump(to, &[]);
                }
            }
            Terminator::Return => {
                // A text an interpolation was building when a `?` left it
                // is freed; the result is the caller's (decision 381).
                for l in self.building.clone().into_iter().rev().filter(|&l| l != 0) {
                    self.free_text(l)?;
                }
                self.leave(abi::OK)
            }
            Terminator::Resume => self.leave(abi::UNWINDING),
            Terminator::Unreachable => {
                self.rt(sym::PANIC_UNREACHABLE, &[])?;
                let fail = self.fail_block();
                self.b.ins().jump(fail, &[]);
            }
        }
        Ok(())
    }

    fn statement(&mut self, s: &StatementKind) -> Result<(), Error> {
        match s {
            StatementKind::Assign(p, r) if p.proj.is_empty() && self.var_of(p.local).is_some() => {
                let dest = self.place_type(p)?;
                let v = self.rvalue(r, &dest)?;
                let w = match &v {
                    Val::Words(ws) if ws.is_empty() => self.iconst(0),
                    _ => self.first_word(&v)?,
                };
                if let Some(var) = self.var_of(p.local) {
                    self.b.def_var(var, w);
                }
                Ok(())
            }
            StatementKind::Assign(p, r) => {
                let dest = self.place_type(p)?;
                let size = self.size(&dest)?;
                let v = self.rvalue(r, &dest)?;
                let (addr, _) = self.dest_place(p)?;
                self.write(addr, &v, size);
                // A copy of a capability handle is counted (decision 237).
                if let Rvalue::Use(Operand::Copy(_)) = r {
                    self.count_at(addr, &dest)?;
                }
                Ok(())
            }
            StatementKind::Call { dest, callee: Callee::Fn(name, tys), args }
                if tys.is_empty()
                    && args.is_empty()
                    && let Some(c) = self.cg.function(name).and_then(returned_constant) =>
            {
                self.constant_read(dest, name, c)
            }
            StatementKind::Call { dest, callee, args } => {
                // A call's result goes to its destination's memory: a
                // variable's slot, read back after the call.
                if dest.proj.is_empty() && self.var_of(dest.local).is_some() {
                    self.reload.push(dest.local.0 as usize);
                }
                self.call(dest, callee, args)
            }
            StatementKind::Drop { place, flag } => self.drop_stmt(place, *flag),
            StatementKind::StorageDead(_) => Ok(()),
        }
    }

    // -------------------------------------------------------------- places

    /// The type of a place, without computing its address.
    fn place_type(&mut self, p: &Place) -> Result<Type, Error> {
        let mut ty = self
            .locals
            .get(p.local.0 as usize)
            .map(|l| l.ty.clone())
            .ok_or_else(|| Error::internal(format!("no local _{}", p.local.0)))?;
        let mut variant = None;
        for proj in &p.proj {
            match proj {
                Proj::Deref => ty = deref_type(&ty)?,
                Proj::Downcast(v) => {
                    variant = Some(*v);
                    continue;
                }
                Proj::Field(i) => {
                    ty = match (&ty, variant) {
                        (Type::Named(n, a), None) if n == "Box" => a.first().cloned().unwrap_or(Type::Unit),
                        _ => self.field_of(&ty, variant, *i)?.1,
                    };
                }
                Proj::Index(_) | Proj::IndexFromEnd(_) => ty = element_type(&ty)?,
                Proj::Key(_) => match &ty {
                    Type::Named(n, a) if n == "dict" => ty = a.get(1).cloned().unwrap_or(Type::Unit),
                    Type::Named(n, a) if n == "Arena" => ty = a.first().cloned().unwrap_or(Type::Unit),
                    other => {
                        return Err(Error::unsupported(format!("a key of a `{}`", sir::write::ty(other))));
                    }
                },
            }
            variant = None;
        }
        Ok(ty)
    }

    /// Field `i` of a value of type `ty` (of `variant`): its offset and
    /// type. In a union of error sets, the members whose variant has the
    /// field must agree on where it is and on its layout: a `String` in one
    /// and a `Path` in another are the same words (decision 211).
    fn field_of(&mut self, ty: &Type, variant: Option<u32>, i: u32) -> Result<(u32, Type), Error> {
        if let Type::ErrSet(names) = ty {
            let text =
                |t: &Type| matches!(t, Type::Named(n, a) if a.is_empty() && (n == "String" || n == "Path"));
            let mut found: Option<(u32, Type)> = None;
            for n in names {
                let t = Type::Named(n.clone(), Vec::new());
                if let Ok(f) = self.cg.types.field(&t, variant, i) {
                    if let Some(g) = &found
                        && (g.0 != f.0 || (g.1 != f.1 && !(text(&g.1) && text(&f.1))))
                    {
                        return Err(Error::unsupported(
                            "a field of a union of error sets whose members differ",
                        ));
                    }
                    found.get_or_insert(f);
                }
            }
            return found.ok_or_else(|| Error::internal("no member of an error union has the field"));
        }
        self.cg.types.field(ty, variant, i)
    }

    /// The address of a place, and its type. With `skip`, a downcast to a
    /// variant the value does not hold jumps there (a drop of a payload
    /// that is not there drops nothing), as does a dict entry that is not
    /// there.
    fn place(&mut self, p: &Place, skip: Option<Block>) -> Result<(Value, Type), Error> {
        self.place_in(p, skip, false)
    }

    /// The destination of an assignment: a dict entry it names is made
    /// when it is not there.
    fn dest_place(&mut self, p: &Place) -> Result<(Value, Type), Error> {
        self.place_in(p, None, true)
    }

    fn place_in(&mut self, p: &Place, skip: Option<Block>, write: bool) -> Result<(Value, Type), Error> {
        let mut ty = self.locals[p.local.0 as usize].ty.clone();
        let mut projs: &[Proj] = &p.proj;
        let mut addr = match self.var_of(p.local) {
            // A reference in a variable: what it refers to is at the
            // address it holds.
            Some(var) if projs.first() == Some(&Proj::Deref) && matches!(ty, Type::Ref(..)) => {
                projs = &projs[1..];
                ty = deref_type(&ty)?;
                self.b.use_var(var)
            }
            // A variable written through its slot is read back after the
            // statement.
            Some(_) => {
                if write && !self.reload.contains(&(p.local.0 as usize)) {
                    self.reload.push(p.local.0 as usize);
                }
                self.local_addr(p.local)?
            }
            None => self.local_addr(p.local)?,
        };
        let mut variant: Option<u32> = None;
        let skipped = p.proj.len() - projs.len();
        for (n, proj) in projs.iter().enumerate() {
            match proj {
                Proj::Deref => {
                    let shared = matches!(&ty, Type::Named(n, _) if n == "Shared");
                    addr = self.through(addr, &ty);
                    if shared {
                        // A task reaching a `Shared` cell counts for `--stats`.
                        self.rt(sym::SHARED_TOUCH, &[addr])?;
                    }
                    ty = deref_type(&ty)?;
                }
                Proj::Downcast(v) => {
                    if let Some(skip) = skip {
                        let off = if self.cg.types.is_error(&ty) { abi::ERROR_TAG } else { 0 };
                        let tag = self.load(addr, off);
                        let is = self.b.ins().icmp_imm(IntCC::Equal, tag, i64::from(*v));
                        let cont = self.b.create_block();
                        self.b.ins().brif(is, cont, &[], skip, &[]);
                        self.b.switch_to_block(cont);
                    }
                    variant = Some(*v);
                    continue;
                }
                Proj::Field(i) => {
                    if let (Type::Named(n, a), None) = (&ty, variant)
                        && n == "Box"
                    {
                        ty = a.first().cloned().unwrap_or(Type::Unit);
                        if write {
                            // A box whose value was moved out gets a new cell
                            // (decision 216).
                            let cell = self.load(addr, 0);
                            let (make, have) = (self.b.create_block(), self.b.create_block());
                            self.b.ins().brif(cell, have, &[], make, &[]);
                            self.b.switch_to_block(make);
                            let size = self.size(&ty)?.max(8);
                            let bytes = self.iconst(i64::from(size));
                            let new = self.rt_status(sym::ALLOC, &[bytes])?;
                            self.store_word(new, addr, 0);
                            self.b.ins().jump(have, &[]);
                            self.b.switch_to_block(have);
                        }
                        addr = self.load(addr, 0);
                    } else {
                        let (off, ft) = self.field_of(&ty, variant, *i)?;
                        if off != 0 {
                            addr = self.b.ins().iadd_imm(addr, i64::from(off));
                        }
                        ty = ft;
                    }
                }
                Proj::Index(l) | Proj::IndexFromEnd(l) => {
                    let elem = element_type(&ty)?;
                    let size = self.size(&elem)?;
                    let from_end = matches!(proj, Proj::IndexFromEnd(_));
                    let (idx, _) = self.scalar(&Operand::Copy(Place::local(*l)))?;
                    let data = self.load(addr, 0);
                    // In the release profile, an index the facts prove in
                    // bounds is not tested again (decision 270).
                    let list = Place { local: p.local, proj: p.proj[..skipped + n].to_vec() };
                    let proven = !from_end
                        && match (&self.facts, &self.known) {
                            (Some(f), Some(st)) => f.in_bounds(st, &list, l.0),
                            _ => false,
                        };
                    if !from_end {
                        self.cg.checks.index += 1;
                        self.cg.checks.index_proven += usize::from(proven);
                    }
                    let at = if from_end {
                        let len = self.load(addr, 8);
                        self.b.ins().isub(len, idx)
                    } else {
                        idx
                    };
                    if !proven {
                        // In bounds: `0 <= i < len`, or `0 < i <= len` from
                        // the end.
                        let len = self.load(addr, 8);
                        let probe = if from_end { self.b.ins().iadd_imm(idx, -1) } else { idx };
                        let bad = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, probe, len);
                        self.guard_if(
                            bad,
                            sym::PANIC_INDEX,
                            [Arg::V(idx), Arg::V(len), Arg::I64(i64::from(from_end))],
                        )?;
                    }
                    let off = self.b.ins().imul_imm(at, i64::from(size));
                    addr = self.b.ins().iadd(data, off);
                    ty = elem;
                }
                Proj::Key(l) if matches!(&ty, Type::Named(n, _) if n == "Arena") => {
                    // An arena's slot, by a key of the right generation.
                    let val = match &ty {
                        Type::Named(_, a) => a.first().cloned().unwrap_or(Type::Unit),
                        _ => Type::Unit,
                    };
                    let mode = if skip.is_some() { abi::SLOT_DROP } else { abi::SLOT_READ };
                    let key = self.local_addr(*l)?;
                    let mv = self.i32const(mode);
                    let out = self.slot(8);
                    let st = self.rt_status(sym::ARENA_SLOT, &[addr, key, mv, out])?;
                    self.check(st);
                    addr = self.load(out, 0);
                    if let Some(skip) = skip {
                        let cont = self.b.create_block();
                        self.b.ins().brif(addr, cont, &[], skip, &[]);
                        self.b.switch_to_block(cont);
                    }
                    ty = val;
                }
                Proj::Key(l) => {
                    let val = match &ty {
                        Type::Named(n, a) if n == "dict" => a.get(1).cloned().unwrap_or(Type::Unit),
                        other => {
                            return Err(Error::unsupported(format!(
                                "a key of a `{}`",
                                sir::write::ty(other)
                            )));
                        }
                    };
                    let last = std::ptr::eq(proj, p.proj.last().unwrap_or(proj));
                    let mode = match (skip, write && last) {
                        (Some(_), _) => abi::SLOT_DROP,
                        (None, true) => abi::SLOT_WRITE,
                        (None, false) => abi::SLOT_READ,
                    };
                    let key = self.local_addr(*l)?;
                    let kt = self.locals[l.0 as usize].ty.clone();
                    let kid = self.type_id(&kt)?;
                    let (kv, mv) = (self.i32const(kid), self.i32const(mode));
                    let out = self.slot(8);
                    let st = self.rt_status(sym::DICT_SLOT, &[addr, key, kv, mv, out])?;
                    self.check(st);
                    addr = self.load(out, 0);
                    if let Some(skip) = skip {
                        let cont = self.b.create_block();
                        self.b.ins().brif(addr, cont, &[], skip, &[]);
                        self.b.switch_to_block(cont);
                    }
                    ty = val;
                }
            }
            variant = None;
        }
        Ok((addr, ty))
    }

    // ------------------------------------------------------------ operands

    fn operand(&mut self, o: &Operand) -> Result<(Val, Type), Error> {
        match o {
            Operand::Const(c) => self.constant(c),
            Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() && self.var_of(p.local).is_some() => {
                let ty = self.locals[p.local.0 as usize].ty.clone();
                let var = self.var_of(p.local).ok_or_else(|| Error::internal("a variable without one"))?;
                Ok((Val::Words(vec![self.b.use_var(var)]), ty))
            }
            Operand::Copy(p) | Operand::Move(p) => {
                let (addr, ty) = self.place(p, None)?;
                let size = self.size(&ty)?;
                let v = self.read(addr, size);
                if matches!(o, Operand::Move(_)) {
                    self.moved_out_of_box(p)?;
                    self.moved_out_of_error(p)?;
                }
                Ok((v, ty))
            }
        }
    }

    /// A value moved out of a `Box` (a local's, or a field's of one) leaves
    /// the box without a value: its cell is freed and the box holds none,
    /// until an assignment gives it one (decision 216).
    fn moved_out_of_box(&mut self, p: &Place) -> Result<(), Error> {
        let Some((Proj::Field(0), base)) = p.proj.split_last() else { return Ok(()) };
        if !base.iter().all(|q| matches!(q, Proj::Field(_))) {
            return Ok(());
        }
        let base = Place { local: p.local, proj: base.to_vec() };
        if !matches!(self.place_type(&base)?, Type::Named(n, _) if n == "Box") {
            return Ok(());
        }
        let (at, _) = self.place(&base, None)?;
        let cell = self.load(at, 0);
        self.rt(sym::FREE, &[cell])?;
        let zero = self.iconst(0);
        self.store_word(zero, at, 0);
        Ok(())
    }

    /// A field moved out of an error value (`e.as(v).k`) leaves the value
    /// never dropped whole: it lets go of its return trace now (decision
    /// 215).
    fn moved_out_of_error(&mut self, p: &Place) -> Result<(), Error> {
        for (i, proj) in p.proj.iter().enumerate() {
            if !matches!(proj, Proj::Downcast(_)) || !matches!(p.proj.get(i + 1), Some(Proj::Field(_))) {
                continue;
            }
            let base = Place { local: p.local, proj: p.proj[..i].to_vec() };
            if !base.proj.iter().all(|q| matches!(q, Proj::Field(_) | Proj::Downcast(_))) {
                continue;
            }
            let ty = self.place_type(&base)?;
            if self.cg.types.is_error(&ty) {
                let (at, _) = self.place(&base, None)?;
                self.rt(sym::TRACE_RELEASE, &[at])?;
            }
        }
        Ok(())
    }

    fn constant(&mut self, c: &Const) -> Result<(Val, Type), Error> {
        Ok(match c {
            Const::Int(n) => (Val::Words(vec![self.iconst(*n)]), Type::Int),
            Const::Float(f) => (Val::Words(vec![self.iconst(f.to_bits() as i64)]), Type::Float),
            Const::Bool(b) => (Val::Words(vec![self.iconst(i64::from(*b))]), Type::Bool),
            Const::Char(ch) => (Val::Words(vec![self.iconst(i64::from(*ch as u32))]), Type::Char),
            Const::Unit => (Val::Words(Vec::new()), Type::Unit),
            Const::Str(s) => {
                let id = self.cg.text(s.as_bytes())?;
                let a = self.data_addr(id);
                let n = self.iconst(s.len() as i64);
                (Val::Words(vec![a, n]), Type::Str)
            }
            Const::Fn(n) => {
                let (thunk, ty) = self.cg.fn_value(n)?;
                let r = self.func_ref(thunk);
                let code = self.b.ins().func_addr(types::I64, r);
                let mut words = vec![code];
                // An empty environment, which holds nothing to count.
                for _ in 0..=self.cg.types.env_words {
                    words.push(self.iconst(0));
                }
                (Val::Words(words), ty)
            }
        })
    }

    /// A one-word operand through any references: its word and type.
    fn scalar(&mut self, o: &Operand) -> Result<(Value, Type), Error> {
        let (v, mut ty) = self.operand(o)?;
        let mut w = match &v {
            Val::Words(ws) if ws.is_empty() => self.iconst(0),
            _ => self.first_word(&v)?,
        };
        while let Type::Ref(inner, _) = ty {
            ty = *inner;
            w = if self.size(&ty)? == 0 { self.iconst(0) } else { self.load(w, 0) };
        }
        Ok((w, ty))
    }

    /// An operand in memory, for the runtime: its address and type.
    fn operand_in_memory(&mut self, o: &Operand) -> Result<(Value, Type), Error> {
        let (v, ty) = self.operand(o)?;
        let size = self.size(&ty)?;
        Ok((self.in_memory(v, size), ty))
    }

    // ------------------------------------------------------------- rvalues

    fn rvalue(&mut self, r: &Rvalue, dest: &Type) -> Result<Val, Error> {
        match r {
            Rvalue::Use(o) => Ok(self.operand(o)?.0),
            Rvalue::Ref(p, _) => {
                let (addr, ty) = self.place(p, None)?;
                if crate::layout::fat(&ty) {
                    // A reference to a `dyn` is the `dyn`'s two words.
                    return Ok(Val::Words(vec![self.load(addr, 0), self.load(addr, 8)]));
                }
                Ok(Val::Words(vec![addr]))
            }
            Rvalue::Binary(op, a, b) => self.binary(*op, a, b, dest),
            Rvalue::Unary(op, a) => self.unary(*op, a),
            Rvalue::Aggregate(kind, ops) => self.aggregate(kind, ops, dest),
            Rvalue::Discriminant(p) => {
                let (mut addr, mut ty) = self.place(p, None)?;
                while let Type::Ref(inner, _) = &ty {
                    addr = self.through(addr, &ty);
                    ty = (**inner).clone();
                }
                let off = if self.cg.types.is_error(&ty) { abi::ERROR_TAG } else { 0 };
                Ok(Val::Words(vec![self.load(addr, off)]))
            }
            Rvalue::Closure(name, tys, ops) => self.closure(name, tys, ops),
            Rvalue::CopyOf(p) => {
                let (mut addr, mut ty) = self.place(p, None)?;
                while let Type::Ref(inner, _) = &ty {
                    if matches!(dest, Type::Ref(..)) {
                        break;
                    }
                    addr = self.through(addr, &ty);
                    ty = (**inner).clone();
                }
                let size = self.size(&ty)?;
                let tmp = self.slot(size);
                let id = self.type_id(&ty)?;
                let idv = self.i32const(id);
                self.rt(sym::COPY, &[tmp, addr, idv])?;
                Ok(Val::Mem(tmp))
            }
            Rvalue::StrView(p) => {
                let (mut addr, mut ty) = self.place(p, None)?;
                while let Type::Ref(inner, _) = &ty {
                    addr = self.through(addr, &ty);
                    ty = (**inner).clone();
                }
                let ptr = self.load(addr, 0);
                let len = self.load(addr, 8);
                Ok(Val::Words(vec![ptr, len]))
            }
        }
    }

    /// A closure value: its function's code, then its environment, the
    /// tuple of its captures, in the value's other words (decision 224).
    /// An `own` closure copies its captures, which are `Copy`, and the
    /// value holds them however far it goes; the program is compiled again
    /// with wider function values when one needs more words than they
    /// have. A closure that captures references cannot leave the frame of
    /// what it captures, so a larger environment of references is a slot
    /// of this frame, whose address is the value's second word. A task's
    /// closure (or a generator's) runs after the statement that makes it,
    /// with an environment the runtime frees when it returns.
    fn closure(&mut self, name: &str, tys: &[Type], ops: &[Operand]) -> Result<Val, Error> {
        let targs: Vec<Type> = tys.iter().map(|t| self.concrete(t)).collect::<Result<_, _>>()?;
        let f = self
            .cg
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let env_ty = match f.locals.get(1) {
            Some(l) => self.cg.concrete(&l.ty, &targs)?,
            None => Type::Tuple(Vec::new()),
        };
        let id = self.cg.instance(name, targs)?;
        let r = self.func_ref(id);
        let code = self.b.ins().func_addr(types::I64, r);
        let size = self.size(&env_ty)?;
        let held = self.cg.types.env_words * abi::WORD;
        let value = self.slot(2 * abi::WORD + held);
        self.store_word(code, value, 0);
        for k in 0..=self.cg.types.env_words {
            let zero = self.iconst(0);
            self.store_word(zero, value, abi::WORD * (k + 1));
        }
        let task = f.kind == sir::FnKind::Closure(sir::ClosureRole::Task);
        let all_refs =
            matches!(&env_ty, Type::Tuple(items) if items.iter().all(|t| matches!(t, Type::Ref(..))));
        let env = if size == 0 {
            return Ok(Val::Mem(value));
        } else if task {
            // The runtime frees it when the closure returns, and forgets
            // what it holds for a generator abandoned before it starts, by
            // its type.
            let bytes = self.iconst(i64::from(size));
            let id = self.type_id(&env_ty)?;
            let idv = self.i32const(id);
            let cell = self.rt_status(sym::TASK_ENV, &[bytes, idv])?;
            self.store_word(cell, value, abi::WORD);
            cell
        } else if size <= held {
            // A drop or a copy of the value reaches the copies an `own`
            // closure holds by the environment's type, in the value's last
            // word (decision 248).
            if !all_refs && self.cg.types.counted(&env_ty)? {
                let id = self.type_id(&env_ty)?;
                let tw = self.iconst(i64::from(id) + 1);
                self.store_word(tw, value, abi::WORD + held);
            }
            self.b.ins().iadd_imm(value, i64::from(abi::WORD))
        } else if all_refs {
            let env = self.slot(size);
            self.store_word(env, value, abi::WORD);
            env
        } else if self.cg.types.holds_fn(&env_ty)? {
            // It keeps a function value, which a wider function value only
            // widens, so no width holds it in place: it goes in a heap cell
            // whose address is the first word, the value's environment a
            // `Box` of it by the last, which a drop frees and a copy copies
            // (decision 382).
            let bytes = self.iconst(i64::from(size));
            let cell = self.rt_status(sym::ALLOC, &[bytes])?;
            self.store_word(cell, value, abi::WORD);
            let boxed = Type::Named("Box".into(), vec![env_ty.clone()]);
            let id = self.type_id(&boxed)?;
            let tw = self.iconst(i64::from(id) + 1);
            self.store_word(tw, value, abi::WORD + held);
            cell
        } else {
            // An `own` environment larger than a function value holds: the
            // program is compiled again with wider ones.
            self.cg.env_needed = self.cg.env_needed.max(size.div_ceil(abi::WORD));
            return Ok(Val::Mem(value));
        };
        for (i, o) in ops.iter().enumerate() {
            let (off, ft) = self.field_of(&env_ty, None, i as u32)?;
            let (v, vt) = self.operand(o)?;
            let fsize = self.size(&vt)?.min(self.size(&ft)?);
            let at = self.b.ins().iadd_imm(env, i64::from(off));
            self.write(at, &v, fsize);
            // A capture of a capability handle is a counted copy (decision
            // 237).
            if matches!(o, Operand::Copy(_)) {
                self.count_at(at, &vt)?;
            }
        }
        Ok(Val::Mem(value))
    }

    fn unary(&mut self, op: UnOp, a: &Operand) -> Result<Val, Error> {
        let (x, ty) = self.scalar(a)?;
        let k = scalar_kind(&ty);
        let r = match (op, k) {
            (UnOp::Neg, Some(Scalar::Int)) => {
                let min = self.b.ins().icmp_imm(IntCC::Equal, x, i64::MIN);
                self.guard_if(
                    min,
                    sym::PANIC_OVERFLOW,
                    [Arg::I32(IntOp::Neg as u32), Arg::V(x), Arg::I64(0)],
                )?;
                self.b.ins().ineg(x)
            }
            (UnOp::Neg, Some(Scalar::Float)) => {
                let f = self.b.ins().bitcast(types::F64, MemFlags::new(), x);
                let n = self.b.ins().fneg(f);
                self.b.ins().bitcast(types::I64, MemFlags::new(), n)
            }
            (UnOp::Neg, Some(Scalar::F32)) => {
                let f = self.f32_of(x);
                let n = self.b.ins().fneg(f);
                self.word_of_f32(n)
            }
            (UnOp::Not, _) => self.b.ins().bxor_imm(x, 1),
            (UnOp::BitNot, _) => self.b.ins().bnot(x),
            (op, _) => {
                return Err(Error::unsupported(format!("`{}` on `{}`", op.name(), sir::write::ty(&ty))));
            }
        };
        Ok(Val::Words(vec![r]))
    }

    fn f32_of(&mut self, w: Value) -> Value {
        let low = self.b.ins().ireduce(types::I32, w);
        self.b.ins().bitcast(types::F32, MemFlags::new(), low)
    }

    fn word_of_f32(&mut self, f: Value) -> Value {
        let bits = self.b.ins().bitcast(types::I32, MemFlags::new(), f);
        self.b.ins().uextend(types::I64, bits)
    }

    fn binary(&mut self, op: BinOp, a: &Operand, b: &Operand, dest: &Type) -> Result<Val, Error> {
        let ta = self.operand_type(a)?;
        let tb = self.operand_type(b)?;
        let (ka, kb) = (scalar_kind(&target_type(&ta)), scalar_kind(&target_type(&tb)));
        let value_op = match op {
            BinOp::Eq => Some(ValueOp::Eq),
            BinOp::Ne => Some(ValueOp::Ne),
            BinOp::Lt => Some(ValueOp::Lt),
            BinOp::Le => Some(ValueOp::Le),
            BinOp::Gt => Some(ValueOp::Gt),
            BinOp::Ge => Some(ValueOp::Ge),
            BinOp::In => Some(ValueOp::In),
            BinOp::Concat => Some(ValueOp::Concat),
            _ => None,
        };
        // `==` and `!=` on two texts: their lengths, then their bytes
        // (decision 253).
        let text = |t: &Type| {
            matches!(t, Type::Str) || matches!(t, Type::Named(n, _) if n == "String" || n == "Path")
        };
        if matches!(op, BinOp::Eq | BinOp::Ne) && text(&target_type(&ta)) && text(&target_type(&tb)) {
            let (pa, ta) = self.operand_in_memory(a)?;
            let (pb, tb) = self.operand_in_memory(b)?;
            let (xa, _) = self.base_of(pa, &ta);
            let (xb, _) = self.base_of(pb, &tb);
            let (la, lb) = (self.load(xa, 8), self.load(xb, 8));
            let same = self.b.ins().icmp(IntCC::Equal, la, lb);
            let (bytes, join) = (self.b.create_block(), self.b.create_block());
            self.b.append_block_param(join, types::I64);
            let zero = self.iconst(0);
            self.b.ins().brif(same, bytes, &[], join, &[zero.into()]);
            self.b.switch_to_block(bytes);
            let (da, db) = (self.load(xa, 0), self.load(xb, 0));
            // The runtime in Sisu's (decision 336).
            let ret = self.rt_sisu(sym::BYTES_EQ, &[da, db, la])?;
            let eq = self.load(ret, 0);
            self.b.ins().jump(join, &[eq.into()]);
            self.b.switch_to_block(join);
            let r = self.b.block_params(join)[0];
            let r = if op == BinOp::Ne { self.b.ins().bxor_imm(r, 1) } else { r };
            return Ok(Val::Words(vec![r]));
        }
        // `==` and `!=` on two values of one enum: their tags, then the
        // variant's fields (decision 253).
        if matches!(op, BinOp::Eq | BinOp::Ne) {
            let xt = target_type(&ta);
            if xt == target_type(&tb)
                && let Some(shape) = self.eq_shape(&xt)?
            {
                let r = self.enum_eq(a, b, &shape)?;
                let r = if op == BinOp::Ne { self.b.ins().bxor_imm(r, 1) } else { r };
                return Ok(Val::Words(vec![r]));
            }
        }
        // Operators on values that are not numbers: the runtime's.
        let scalars = ka.is_some() && kb.is_some();
        if let Some(vop) = value_op
            && (!scalars || matches!(vop, ValueOp::In | ValueOp::Concat))
        {
            let (pa, ta) = self.operand_in_memory(a)?;
            let (pb, tb) = self.operand_in_memory(b)?;
            let size = self.size(dest)?;
            let out = self.slot(size);
            let (ia, ib) = (self.type_id(&ta)?, self.type_id(&tb)?);
            let code = self.i32const(vop as u32);
            let (ia, ib) = (self.i32const(ia), self.i32const(ib));
            let st = self.rt_status(sym::BINARY, &[code, out, pa, ia, pb, ib])?;
            self.check(st);
            return Ok(self.read(out, size));
        }
        let (x, tx) = self.scalar(a)?;
        let (y, _) = self.scalar(b)?;
        let k = scalar_kind(&tx).ok_or_else(|| Error::internal("a scalar operator on a value"))?;
        if let Some(vop) = value_op {
            let r = self.compare(vop, k, x, y);
            let r = self.b.ins().uextend(types::I64, r);
            return Ok(Val::Words(vec![r]));
        }
        let r = match k {
            Scalar::Int => {
                // In the release profile, an operation whose operands'
                // bounds rule out an overflow is the plain instruction
                // (decision 270).
                let checked = match (&self.facts, &self.known) {
                    (Some(f), Some(st)) => !f.cannot_overflow(st, op, a, b),
                    _ => true,
                };
                if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
                    self.cg.checks.overflow += 1;
                    self.cg.checks.overflow_proven += usize::from(!checked);
                }
                self.int_op(op, x, y, checked)?
            }
            Scalar::Float => {
                let (fx, fy) = (
                    self.b.ins().bitcast(types::F64, MemFlags::new(), x),
                    self.b.ins().bitcast(types::F64, MemFlags::new(), y),
                );
                let r = self.float_op(op, fx, fy)?;
                self.b.ins().bitcast(types::I64, MemFlags::new(), r)
            }
            Scalar::F32 => {
                // Computed in `f64` and rounded once (decision 169).
                let (fx, fy) = (self.f32_of(x), self.f32_of(y));
                let (dx, dy) = (self.b.ins().fpromote(types::F64, fx), self.b.ins().fpromote(types::F64, fy));
                let r = self.float_op(op, dx, dy)?;
                let r = self.b.ins().fdemote(types::F32, r);
                self.word_of_f32(r)
            }
            _ => {
                return Err(Error::unsupported(format!("`{}` on `{}`", op.name(), sir::write::ty(&tx))));
            }
        };
        Ok(Val::Words(vec![r]))
    }

    /// How `==` compares each variant of `t`, when `t` is an `Option` or
    /// a declared enum other than an error set's; `None` for any other
    /// type.
    fn eq_shape(&mut self, t: &Type) -> Result<Option<Vec<Same>>, Error> {
        let Type::Named(n, args) = t else { return Ok(None) };
        let variants: Vec<Vec<Type>> = if n == "Option" {
            vec![args.first().cloned().into_iter().collect(), Vec::new()]
        } else {
            match self.cg.types.decl(n) {
                Some(sir::TypeDecl::Enum { variants, error: false, .. }) => {
                    variants.iter().map(|v| v.fields.iter().map(|(_, ft)| ft.subst(args)).collect()).collect()
                }
                _ => return Ok(None),
            }
        };
        let mut shape = Vec::new();
        for fields in variants {
            let (mut off, mut words, mut runtime) = (abi::WORD, Vec::new(), false);
            for ft in &fields {
                let size = self.size(ft)?;
                if self.word_compared(ft) {
                    words.push(off);
                } else if size > 0 {
                    runtime = true;
                }
                off += size;
            }
            shape.push(match (runtime, words.is_empty()) {
                (true, _) => Same::Runtime,
                (false, true) => Same::Tag,
                (false, false) => Same::Words(words),
            });
        }
        Ok(Some(shape))
    }

    /// Whether the runtime compares two values of type `t` by their one
    /// word: an integer, a `bool`, a `char`, or an enum without fields.
    fn word_compared(&self, t: &Type) -> bool {
        match t {
            Type::Int | Type::Bool | Type::Char => true,
            Type::Named(n, _)
                if matches!(n.as_str(), "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32") =>
            {
                true
            }
            Type::Named(n, _) => matches!(
                self.cg.types.decl(n),
                Some(sir::TypeDecl::Enum { variants, error: false, .. }) if variants.iter().all(|v| v.fields.is_empty())
            ),
            _ => false,
        }
    }

    /// `==` on two values of one enum, whose variants compare as `shape`
    /// says: a word, 1 when they are equal. Fields the runtime compares
    /// are its, as `sisu_rt_binary` would.
    fn enum_eq(&mut self, a: &Operand, b: &Operand, shape: &[Same]) -> Result<Value, Error> {
        let (pa, ta) = self.operand_in_memory(a)?;
        let (pb, tb) = self.operand_in_memory(b)?;
        let (xa, _) = self.base_of(pa, &ta);
        let (xb, _) = self.base_of(pb, &tb);
        let (ga, gb) = (self.load(xa, 0), self.load(xb, 0));
        let same = self.b.ins().icmp(IntCC::Equal, ga, gb);
        if shape.iter().all(|s| matches!(s, Same::Tag)) {
            return Ok(self.b.ins().uextend(types::I64, same));
        }
        let (fields, yes, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        self.b.append_block_param(join, types::I64);
        let zero = self.iconst(0);
        self.b.ins().brif(same, fields, &[], join, &[zero.into()]);
        self.b.switch_to_block(fields);
        let mut sw = Switch::new();
        let mut by_words = Vec::new();
        let mut runtime = None;
        for (v, s) in shape.iter().enumerate() {
            match s {
                Same::Tag => {}
                Same::Words(offs) => {
                    let blk = self.b.create_block();
                    sw.set_entry(v as u128, blk);
                    by_words.push((blk, offs));
                }
                Same::Runtime => {
                    let blk = *runtime.get_or_insert_with(|| self.b.create_block());
                    sw.set_entry(v as u128, blk);
                }
            }
        }
        sw.emit(&mut self.b, ga, yes);
        self.b.switch_to_block(yes);
        let one = self.iconst(1);
        self.b.ins().jump(join, &[one.into()]);
        for (blk, offs) in by_words {
            self.b.switch_to_block(blk);
            let mut all = None;
            for &off in offs {
                let (wa, wb) = (self.load(xa, off), self.load(xb, off));
                let e = self.b.ins().icmp(IntCC::Equal, wa, wb);
                all = Some(match all {
                    Some(x) => self.b.ins().band(x, e),
                    None => e,
                });
            }
            let r = match all {
                Some(x) => self.b.ins().uextend(types::I64, x),
                None => self.iconst(1),
            };
            self.b.ins().jump(join, &[r.into()]);
        }
        if let Some(blk) = runtime {
            self.b.switch_to_block(blk);
            let out = self.slot(abi::WORD);
            let (ia, ib) = (self.type_id(&ta)?, self.type_id(&tb)?);
            let code = self.i32const(ValueOp::Eq as u32);
            let (ia, ib) = (self.i32const(ia), self.i32const(ib));
            let st = self.rt_status(sym::BINARY, &[code, out, pa, ia, pb, ib])?;
            self.check(st);
            let r = self.load(out, 0);
            self.b.ins().jump(join, &[r.into()]);
        }
        self.b.switch_to_block(join);
        Ok(self.b.block_params(join)[0])
    }

    fn operand_type(&mut self, o: &Operand) -> Result<Type, Error> {
        Ok(match o {
            Operand::Const(c) => match c {
                Const::Int(_) => Type::Int,
                Const::Float(_) => Type::Float,
                Const::Bool(_) => Type::Bool,
                Const::Char(_) => Type::Char,
                Const::Unit => Type::Unit,
                Const::Str(_) => Type::Str,
                Const::Fn(n) => self.cg.fn_value(n)?.1,
            },
            Operand::Copy(p) | Operand::Move(p) => self.place_type(p)?,
        })
    }

    /// A comparison of two scalars of kind `k`, as a `bool` byte.
    fn compare(&mut self, op: ValueOp, k: Scalar, x: Value, y: Value) -> Value {
        match k {
            Scalar::Float | Scalar::F32 => {
                let (fx, fy) = if k == Scalar::Float {
                    (
                        self.b.ins().bitcast(types::F64, MemFlags::new(), x),
                        self.b.ins().bitcast(types::F64, MemFlags::new(), y),
                    )
                } else {
                    (self.f32_of(x), self.f32_of(y))
                };
                let cc = match op {
                    ValueOp::Eq => FloatCC::Equal,
                    ValueOp::Ne => FloatCC::NotEqual,
                    ValueOp::Lt => FloatCC::LessThan,
                    ValueOp::Le => FloatCC::LessThanOrEqual,
                    ValueOp::Gt => FloatCC::GreaterThan,
                    _ => FloatCC::GreaterThanOrEqual,
                };
                self.b.ins().fcmp(cc, fx, fy)
            }
            Scalar::Unit => {
                let v = matches!(op, ValueOp::Eq | ValueOp::Le | ValueOp::Ge);
                self.b.ins().iconst(types::I8, i64::from(v))
            }
            _ => {
                let signed = k == Scalar::Int;
                let cc = match (op, signed) {
                    (ValueOp::Eq, _) => IntCC::Equal,
                    (ValueOp::Ne, _) => IntCC::NotEqual,
                    (ValueOp::Lt, true) => IntCC::SignedLessThan,
                    (ValueOp::Le, true) => IntCC::SignedLessThanOrEqual,
                    (ValueOp::Gt, true) => IntCC::SignedGreaterThan,
                    (_, true) => IntCC::SignedGreaterThanOrEqual,
                    (ValueOp::Lt, false) => IntCC::UnsignedLessThan,
                    (ValueOp::Le, false) => IntCC::UnsignedLessThanOrEqual,
                    (ValueOp::Gt, false) => IntCC::UnsignedGreaterThan,
                    (_, false) => IntCC::UnsignedGreaterThanOrEqual,
                };
                self.b.ins().icmp(cc, x, y)
            }
        }
    }

    /// An operation on two `int`s; `+`, `-` and `*` test the processor's
    /// overflow flag (Cranelift's `sadd_overflow` and the others) and
    /// branch to a cold block that raises the panic, unless `checked` is
    /// false, when the facts prove it cannot overflow (decision 270).
    fn int_op(&mut self, op: BinOp, x: Value, y: Value, checked: bool) -> Result<Value, Error> {
        let overflowing = |op: BinOp| match op {
            BinOp::Add => Some(IntOp::Add),
            BinOp::Sub => Some(IntOp::Sub),
            BinOp::Mul => Some(IntOp::Mul),
            _ => None,
        };
        if let Some(iop) = overflowing(op) {
            if !checked {
                return Ok(match iop {
                    IntOp::Add => self.b.ins().iadd(x, y),
                    IntOp::Sub => self.b.ins().isub(x, y),
                    _ => self.b.ins().imul(x, y),
                });
            }
            let (r, of) = match iop {
                IntOp::Add => self.b.ins().sadd_overflow(x, y),
                IntOp::Sub => self.b.ins().ssub_overflow(x, y),
                _ => self.b.ins().smul_overflow(x, y),
            };
            self.guard_if(of, sym::PANIC_OVERFLOW, [Arg::I32(iop as u32), Arg::V(x), Arg::V(y)])?;
            return Ok(r);
        }
        let runtime = match op {
            BinOp::Div | BinOp::FloorDiv => Some(IntOp::FloorDiv),
            BinOp::Mod => Some(IntOp::Mod),
            BinOp::Pow => Some(IntOp::Pow),
            BinOp::Shl => Some(IntOp::Shl),
            BinOp::Shr => Some(IntOp::Shr),
            _ => None,
        };
        if let Some(iop) = runtime {
            if iop != IntOp::Pow {
                return self.inline_int_op(iop, x, y);
            }
            return self.runtime_int_op(iop, x, y);
        }
        Ok(match op {
            BinOp::WrapAdd => self.b.ins().iadd(x, y),
            BinOp::WrapSub => self.b.ins().isub(x, y),
            BinOp::WrapMul => self.b.ins().imul(x, y),
            BinOp::SatAdd | BinOp::SatSub | BinOp::SatMul => {
                let (r, of) = match op {
                    BinOp::SatAdd => self.b.ins().sadd_overflow(x, y),
                    BinOp::SatSub => self.b.ins().ssub_overflow(x, y),
                    _ => self.b.ins().smul_overflow(x, y),
                };
                // The bound the true result passed: by `y`'s sign for `+`
                // and `-`, by whether the signs differ for `*`.
                let neg = match op {
                    BinOp::SatAdd => self.b.ins().icmp_imm(IntCC::SignedLessThan, y, 0),
                    BinOp::SatSub => self.b.ins().icmp_imm(IntCC::SignedGreaterThan, y, 0),
                    _ => {
                        let s = self.b.ins().bxor(x, y);
                        self.b.ins().icmp_imm(IntCC::SignedLessThan, s, 0)
                    }
                };
                let (min, max) = (self.iconst(i64::MIN), self.iconst(i64::MAX));
                let bound = self.b.ins().select(neg, min, max);
                self.b.ins().select(of, bound, r)
            }
            BinOp::BitAnd => self.b.ins().band(x, y),
            BinOp::BitOr => self.b.ins().bor(x, y),
            BinOp::BitXor => self.b.ins().bxor(x, y),
            other => return Err(Error::unsupported(format!("`{}` on integers", other.name()))),
        })
    }

    /// The runtime's integer operation (`sisu_rt_int_op`): its result, or
    /// its panic.
    fn runtime_int_op(&mut self, iop: IntOp, x: Value, y: Value) -> Result<Value, Error> {
        let out = self.slot(8);
        let code = self.i32const(iop as u32);
        let st = self.rt_status(sym::INT_OP, &[code, x, y, out])?;
        self.check(st);
        Ok(self.load(out, 0))
    }

    /// `//`, `%`, `<<` and `>>` as instructions (decision 246), and, on a
    /// cold path, as the runtime's call for the operands it raises a panic
    /// for: a divisor of zero, `MIN // -1` (which does not fit; `MIN % -1`
    /// is 0, which the runtime gives), and a shift by less than 0 or more
    /// than 63. The quotient rounds down and the remainder takes the
    /// divisor's sign, as the runtime's do.
    fn inline_int_op(&mut self, iop: IntOp, x: Value, y: Value) -> Result<Value, Error> {
        let slow_case = match iop {
            IntOp::Shl | IntOp::Shr => self.b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, y, 64),
            _ => {
                let zero = self.b.ins().icmp_imm(IntCC::Equal, y, 0);
                let minus_one = self.b.ins().icmp_imm(IntCC::Equal, y, -1);
                let min = self.b.ins().icmp_imm(IntCC::Equal, x, i64::MIN);
                let overflow = self.b.ins().band(minus_one, min);
                self.b.ins().bor(zero, overflow)
            }
        };
        let (fast, slow, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        self.b.append_block_param(join, types::I64);
        self.b.ins().brif(slow_case, slow, &[], fast, &[]);
        self.b.switch_to_block(fast);
        let v = match iop {
            IntOp::Shl => self.b.ins().ishl(x, y),
            IntOp::Shr => self.b.ins().sshr(x, y),
            IntOp::Mod => {
                let r = self.b.ins().srem(x, y);
                let signs = self.b.ins().bxor(r, y);
                let differ = self.b.ins().icmp_imm(IntCC::SignedLessThan, signs, 0);
                let nonzero = self.b.ins().icmp_imm(IntCC::NotEqual, r, 0);
                let adjust = self.b.ins().band(differ, nonzero);
                let up = self.b.ins().iadd(r, y);
                self.b.ins().select(adjust, up, r)
            }
            _ => {
                let q = self.b.ins().sdiv(x, y);
                let qy = self.b.ins().imul(q, y);
                let r = self.b.ins().isub(x, qy);
                let signs = self.b.ins().bxor(r, y);
                let differ = self.b.ins().icmp_imm(IntCC::SignedLessThan, signs, 0);
                let nonzero = self.b.ins().icmp_imm(IntCC::NotEqual, r, 0);
                let adjust = self.b.ins().band(differ, nonzero);
                let one = self.b.ins().uextend(types::I64, adjust);
                self.b.ins().isub(q, one)
            }
        };
        self.b.ins().jump(join, &[v.into()]);
        self.b.set_cold_block(slow);
        self.b.switch_to_block(slow);
        let v = self.runtime_int_op(iop, x, y)?;
        self.b.ins().jump(join, &[v.into()]);
        self.b.switch_to_block(join);
        Ok(self.b.block_params(join)[0])
    }

    fn float_op(&mut self, op: BinOp, x: Value, y: Value) -> Result<Value, Error> {
        Ok(match op {
            BinOp::Add => self.b.ins().fadd(x, y),
            BinOp::Sub => self.b.ins().fsub(x, y),
            BinOp::Mul => self.b.ins().fmul(x, y),
            BinOp::Div => self.b.ins().fdiv(x, y),
            BinOp::FloorDiv | BinOp::Mod | BinOp::Pow => {
                let code = match op {
                    BinOp::Mod => abi::FLOAT_MOD,
                    BinOp::Pow => abi::FLOAT_POW,
                    _ => abi::FLOAT_FLOORDIV,
                };
                let code = self.i32const(code);
                self.rt_status(sym::FLOAT_OP, &[code, x, y])?
            }
            other => return Err(Error::unsupported(format!("`{}` on floats", other.name()))),
        })
    }

    fn aggregate(&mut self, kind: &Aggregate, ops: &[Operand], dest: &Type) -> Result<Val, Error> {
        let size = self.size(dest)?;
        let out = self.slot(size);
        match kind {
            Aggregate::Tuple | Aggregate::Struct(_) => {
                if let Type::Named(n, a) = dest
                    && n == "Box"
                {
                    let inner = a.first().cloned().unwrap_or(Type::Unit);
                    let isize = self.size(&inner)?;
                    let bytes = self.iconst(i64::from(isize));
                    let cell = self.rt_status(sym::ALLOC, &[bytes])?;
                    if let Some(o) = ops.first() {
                        let (v, vt) = self.operand(o)?;
                        self.write(cell, &v, isize);
                        // A copy's handles are counted, as a field's are
                        // (decision 462).
                        if matches!(o, Operand::Copy(_)) {
                            self.count_at(cell, &vt)?;
                        }
                    }
                    self.store_word(cell, out, 0);
                    return Ok(Val::Mem(out));
                }
                for (i, o) in ops.iter().enumerate() {
                    let (off, ft) = self.field_of(dest, None, i as u32)?;
                    let (v, vt) = self.operand(o)?;
                    let fsize = self.size(&vt)?.min(self.size(&ft)?);
                    let at = self.b.ins().iadd_imm(out, i64::from(off));
                    self.write(at, &v, fsize);
                    if matches!(o, Operand::Copy(_)) {
                        self.count_at(at, &vt)?;
                    }
                }
            }
            Aggregate::Variant(t, v) => {
                let index = self
                    .cg
                    .types
                    .variant_index(t, v)
                    .ok_or_else(|| Error::internal(format!("no variant {t}::{v}")))?;
                // The enum's own type: an error set's value is its
                // declaration's.
                let ety = match dest {
                    Type::ErrSet(_) => Type::Named(t.clone(), Vec::new()),
                    other => other.clone(),
                };
                let error = self.cg.types.is_error(&ety);
                if error {
                    let id = self.type_id(&ety)?;
                    let idv = self.iconst(i64::from(id));
                    self.store_word(idv, out, abi::ERROR_DECL);
                    let zero = self.iconst(0);
                    self.store_word(zero, out, abi::ERROR_TRACE);
                }
                let tag = self.iconst(i64::from(index));
                self.store_word(tag, out, if error { abi::ERROR_TAG } else { 0 });
                for (i, o) in ops.iter().enumerate() {
                    let (off, ft) = self.cg.types.field(&ety, Some(index), i as u32)?;
                    let (val, vt) = self.operand(o)?;
                    let fsize = self.size(&vt)?.min(self.size(&ft)?);
                    let at = self.b.ins().iadd_imm(out, i64::from(off));
                    self.write(at, &val, fsize);
                    if matches!(o, Operand::Copy(_)) {
                        self.count_at(at, &vt)?;
                    }
                }
            }
            Aggregate::List => {
                let elem = element_type(dest)?;
                let esize = self.size(&elem)?;
                let n = ops.len() as i64;
                let bytes = self.iconst(n * i64::from(esize));
                let data = self.rt_status(sym::ALLOC, &[bytes])?;
                for (i, o) in ops.iter().enumerate() {
                    let (v, vt) = self.operand(o)?;
                    let at = self.b.ins().iadd_imm(data, i as i64 * i64::from(esize));
                    self.write(at, &v, esize);
                    if matches!(o, Operand::Copy(_)) {
                        self.count_at(at, &vt)?;
                    }
                }
                let len = self.iconst(n);
                self.store_word(data, out, 0);
                self.store_word(len, out, 8);
                self.store_word(len, out, 16);
            }
            Aggregate::Set | Aggregate::Dict => {
                // The entries in a buffer, then a table made of them.
                let (key, val) = match dest {
                    Type::Named(n, a) if n == "dict" => {
                        (a.first().cloned().unwrap_or(Type::Unit), a.get(1).cloned().unwrap_or(Type::Unit))
                    }
                    Type::Named(_, a) => (a.first().cloned().unwrap_or(Type::Unit), Type::Unit),
                    other => {
                        return Err(Error::internal(format!(
                            "a table literal of type `{}`",
                            sir::write::ty(other)
                        )));
                    }
                };
                let (ks, vs) = (self.size(&key)?, self.size(&val)?);
                let per = if matches!(kind, Aggregate::Dict) { 2 } else { 1 };
                let n = ops.len() / per;
                let buf = self.slot((ks + vs) * n as u32);
                for (i, o) in ops.iter().enumerate() {
                    let entry = (i / per) as u32 * (ks + vs);
                    let (off, size) = if i % per == 0 { (entry, ks) } else { (entry + ks, vs) };
                    let (v, vt) = self.operand(o)?;
                    let size = size.min(self.size(&vt)?);
                    let at = self.b.ins().iadd_imm(buf, i64::from(off));
                    self.write(at, &v, size);
                }
                let id = self.type_id(dest)?;
                let (idv, nv) = (self.i32const(id), self.iconst(n as i64));
                let st = self.rt_status(sym::TABLE, &[idv, buf, nv, out])?;
                self.check(st);
            }
            Aggregate::Dyn(t) => {
                // The type of the value and its address: a heap cell the
                // `dyn` owns, or for `dyn ref T(r)` (a `ref dyn` made from a
                // `ref T`) the address `r` holds.
                let carried = self.concrete(t)?;
                let op = ops.first().ok_or_else(|| Error::internal("a `dyn` without its value"))?;
                let (v, _) = self.operand(op)?;
                let (value_ty, at) = match &carried {
                    Type::Ref(inner, _) => ((**inner).clone(), self.first_word(&v)?),
                    _ => {
                        let size = self.size(&carried)?;
                        let bytes = self.iconst(i64::from(size));
                        let cell = self.rt_status(sym::ALLOC, &[bytes])?;
                        self.write(cell, &v, size);
                        // A copy's handles are counted, as a field's are
                        // (decision 462).
                        if matches!(op, Operand::Copy(_)) {
                            self.count_at(cell, &carried)?;
                        }
                        (carried.clone(), cell)
                    }
                };
                if matches!(value_ty, Type::Dyn(..)) {
                    return Err(Error::unsupported("a `dyn` of a `dyn`"));
                }
                let dyn_ty = match dest {
                    Type::Ref(inner, _) => (**inner).clone(),
                    other => other.clone(),
                };
                let id = self.type_id(&value_ty)?;
                self.cg.carry(&dyn_ty, &value_ty)?;
                let idv = self.iconst(i64::from(id));
                self.store_word(idv, out, 0);
                self.store_word(at, out, 8);
            }
        }
        Ok(Val::Mem(out))
    }

    // --------------------------------------------------------------- drops

    fn drop_stmt(&mut self, place: &Place, flag: Option<sir::LocalId>) -> Result<(), Error> {
        // An iterator in the frame holds nothing to free.
        if place.proj.is_empty() && self.frame_of(place.local).is_some() {
            return Ok(());
        }
        let ty = self.place_type(place)?;
        if !self.cg.types.needs_drop(&ty)? {
            return Ok(());
        }
        let done = self.b.create_block();
        if let Some(fl) = flag {
            let var = self.var_of(fl);
            let a = match var {
                Some(_) => None,
                None => Some(self.local_addr(fl)?),
            };
            let set = match (var, a) {
                (Some(v), _) => self.b.use_var(v),
                (None, Some(a)) => self.load(a, 0),
                (None, None) => self.iconst(0),
            };
            let go = self.b.create_block();
            self.b.ins().brif(set, go, &[], done, &[]);
            self.b.switch_to_block(go);
            let zero = self.iconst(0);
            match (var, a) {
                (Some(v), _) => self.b.def_var(v, zero),
                (None, Some(a)) => self.store_word(zero, a, 0),
                (None, None) => {}
            }
        }
        let (addr, ty) = self.place(place, Some(done))?;
        // A function value drops what its environment holds, when its last
        // word names the environment's type (decision 248).
        if let Some((tw, skip)) = self.fn_env(addr, &ty)? {
            let at = self.b.ins().iadd_imm(addr, i64::from(abi::WORD));
            let t = self.b.ins().iadd_imm(tw, -1);
            let t = self.b.ins().ireduce(types::I32, t);
            let st = self.rt_status(sym::DROP, &[at, t])?;
            self.check(st);
            self.b.ins().jump(skip, &[]);
            self.b.switch_to_block(skip);
            self.b.ins().jump(done, &[]);
            self.b.switch_to_block(done);
            return Ok(());
        }
        // A text, or a list of values with nothing to drop, frees its
        // buffer, if it has one, and is left empty, as the runtime leaves
        // it (decision 253).
        let plain_list = match &ty {
            Type::Named(n, a) if n == "list" => match a.first() {
                Some(e) => !self.cg.types.needs_drop(e)?,
                None => false,
            },
            _ => false,
        };
        if plain_list || matches!(&ty, Type::Named(n, _) if n == "String" || n == "Path" || n == "bytes") {
            let ptr = self.load(addr, 0);
            let free = self.b.create_block();
            self.b.ins().brif(ptr, free, &[], done, &[]);
            self.b.switch_to_block(free);
            self.rt(sym::FREE, &[ptr])?;
            let zero = self.iconst(0);
            for off in [0, 8, 16] {
                self.store_word(zero, addr, off);
            }
            self.b.ins().jump(done, &[]);
            self.b.switch_to_block(done);
            return Ok(());
        }
        let id = self.type_id(&ty)?;
        let idv = self.i32const(id);
        let st = self.rt_status(sym::DROP, &[addr, idv])?;
        self.check(st);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
        Ok(())
    }

    // --------------------------------------------------------------- calls

    fn call(&mut self, dest: &Place, callee: &Callee, args: &[Operand]) -> Result<(), Error> {
        match callee {
            Callee::Fn(name, tys) => {
                let targs: Vec<Type> = tys.iter().map(|t| self.concrete(t)).collect::<Result<_, _>>()?;
                self.call_fn(dest, name, targs, args, 0, false)
            }
            Callee::Trait(name, tys) => {
                let tys: Vec<Type> = tys.iter().map(|t| self.concrete(t)).collect::<Result<_, _>>()?;
                let (target, derefs) = self.cg.resolve_trait(name, tys)?;
                match target {
                    Target::Fn(f, targs) => self.call_fn(dest, &f, targs, args, derefs, true),
                    Target::Intrinsic(i, targs) => self.call_intrinsic(dest, &i, &targs, args, derefs),
                    Target::Dyn(m, tys) => self.call_dyn(dest, &m, tys, args, derefs),
                }
            }
            Callee::Intrinsic(name, tys) => {
                let targs: Vec<Type> = tys.iter().map(|t| self.concrete(t)).collect::<Result<_, _>>()?;
                self.call_intrinsic(dest, name, &targs, args, 0)
            }
            Callee::Value(f) => self.call_value(dest, f, args),
        }
    }

    /// The arguments of a call, each in a slot of its own (the callee owns
    /// its arguments' memory): the first through `derefs` references (a
    /// trait's `Self` behind references), and, with `params`, each that
    /// the callee takes by value read through its reference (decision 59).
    fn arguments(
        &mut self,
        args: &[Operand],
        derefs: usize,
        params: Option<&[Type]>,
    ) -> Result<Vec<(Value, Type)>, Error> {
        let mut out = Vec::new();
        for (i, o) in args.iter().enumerate() {
            let (v, mut ty) = self.operand(o)?;
            let size = self.size(&ty)?;
            let mut addr = self.in_memory(v, size);
            if i == 0 {
                for _ in 0..derefs {
                    let Type::Ref(inner, _) = &ty else { break };
                    let inner = (**inner).clone();
                    let p = self.load(addr, 0);
                    let s = self.size(&inner)?;
                    let tmp = self.slot(s);
                    self.copy_bytes(tmp, p, s);
                    addr = tmp;
                    ty = inner;
                }
            }
            if let Some(ps) = params
                && let Some(pt) = ps.get(i)
                && !matches!(pt, Type::Ref(..))
                && let Type::Ref(inner, _) = &ty
                && **inner == *pt
            {
                let p = self.load(addr, 0);
                let s = self.size(pt)?;
                let tmp = self.slot(s);
                self.copy_bytes(tmp, p, s);
                addr = tmp;
                ty = pt.clone();
            }
            out.push((addr, ty));
        }
        Ok(out)
    }

    /// A call of a trait's method on a `dyn`: its dispatcher, given the
    /// address of the `dyn`'s two words (the type of its value and the
    /// value's address) and the other arguments. A capability handle that
    /// may hold any object (its type is `Self`, decision 209) is the
    /// address of such two words.
    fn call_dyn(
        &mut self,
        dest: &Place,
        name: &str,
        tys: Vec<Type>,
        args: &[Operand],
        derefs: usize,
    ) -> Result<(), Error> {
        let first = args.first().ok_or_else(|| Error::internal(format!("`{name}` without `self`")))?;
        let (v, ty) = self.operand(first)?;
        let addr = if matches!(tys.first(), Some(Type::Named(..))) {
            // `self` is a reference to the handle, behind `derefs` more.
            let mut w = self.first_word(&v)?;
            for _ in 0..=derefs {
                w = self.load(w, 0);
            }
            w
        } else {
            let size = self.size(&ty)?;
            let mut addr = self.in_memory(v, size);
            // `ref ... ref dyn`: the innermost reference is the `dyn`'s words.
            for _ in 1..derefs {
                addr = self.load(addr, 0);
            }
            addr
        };
        let rest = self.arguments(&args[1..], 0, None)?;
        // The method owns its arguments: copies of handles are counted
        // here, and a by-value `self` by the dispatcher (decision 237).
        for (o, (a, t)) in args[1..].iter().zip(&rest) {
            if !matches!(o, Operand::Move(_)) && !matches!(t, Type::Ref(..)) {
                self.count_at(*a, t)?;
            }
        }
        let ret_ty = self.place_type(dest)?;
        let arg_tys: Vec<Type> = rest.iter().map(|(_, t)| t.clone()).collect();
        let id = self.cg.dispatcher(name, tys, arg_tys, ret_ty.clone())?;
        let rsize = self.size(&ret_ty)?;
        let ret = self.slot(rsize);
        let mut call_args = vec![ret, addr];
        call_args.extend(rest.iter().map(|(a, _)| *a));
        let r = self.func_ref(id);
        let call = self.b.ins().call(r, &call_args);
        let st = self.b.inst_results(call)[0];
        // A `dyn` moved into the call (`own self`) gave its value to the
        // method: its cell is empty (decision 216).
        if matches!(ty, Type::Dyn(..)) {
            let cell = self.load(addr, 8);
            self.rt(sym::FREE, &[cell])?;
        }
        self.check(st);
        let (daddr, _) = self.place(dest, None)?;
        self.copy_bytes(daddr, ret, rsize);
        Ok(())
    }

    fn call_fn(
        &mut self,
        dest: &Place,
        name: &str,
        targs: Vec<Type>,
        args: &[Operand],
        derefs: usize,
        adapt: bool,
    ) -> Result<(), Error> {
        let f = self
            .cg
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let mut params = Vec::new();
        for i in 0..f.param_count as usize {
            params.push(self.cg.concrete(&f.locals[i + 1].ty, &targs)?);
        }
        let ret_ty = self.cg.concrete(f.ret(), &targs)?;
        let id = self.cg.instance(name, targs)?;
        let mut argv = self.arguments(args, derefs, adapt.then_some(params.as_slice()))?;
        // The callee owns its arguments: copies of handles, and a value
        // read through a reference for a parameter that takes it, are
        // counted (decision 237).
        for (i, (a, t)) in argv.iter().enumerate() {
            if matches!(args.get(i), Some(Operand::Move(_))) {
                continue;
            }
            let p = params.get(i).cloned().unwrap_or_else(|| t.clone());
            if !matches!(p, Type::Ref(..)) {
                self.count_at(*a, &p)?;
            }
        }
        // An `Iterable[T]` parameter takes a reference to what the caller
        // passed, with its type (decision 210); a value moved in is the
        // callee's to read, and dropped when the call returns.
        let mut lent = Vec::new();
        for (a, p) in argv.iter_mut().zip(&params) {
            let Type::Ref(want, _) = p else { continue };
            if !matches!(&**want, Type::Named(n, _) if n == "Iterable") || a.1 == *p {
                continue;
            }
            let (have, ptr) = match a.1.clone() {
                Type::Ref(have, _) => (*have, self.load(a.0, 0)),
                owned => {
                    lent.push((a.0, owned.clone()));
                    (owned, a.0)
                }
            };
            let id = self.type_id(&have)?;
            let fat = self.slot(16);
            let idv = self.iconst(i64::from(id));
            self.store_word(idv, fat, 0);
            self.store_word(ptr, fat, 8);
            *a = (fat, p.clone());
        }
        let rsize = self.size(&ret_ty)?;
        let ret = self.slot(rsize);
        let mut call_args = vec![ret];
        call_args.extend(argv.iter().map(|(a, _)| *a));
        let r = self.func_ref(id);
        let call = self.b.ins().call(r, &call_args);
        let st = self.b.inst_results(call)[0];
        let mut dropped = Vec::new();
        for (addr, ty) in lent {
            if self.cg.types.needs_drop(&ty)? {
                dropped.push((addr, self.type_id(&ty)?));
            }
        }
        if dropped.is_empty() || self.f.blocks[self.cur].cleanup {
            self.check(st);
        } else {
            // A panic leaving the callee drops what it was lent too, as
            // its unwinding goes on (decision 235): a drop that panics
            // then aborts, as one in a cleanup block does.
            let failed = self.b.create_block();
            let cont = self.b.create_block();
            self.b.ins().brif(st, failed, &[], cont, &[]);
            self.b.switch_to_block(failed);
            for &(addr, t) in &dropped {
                let idv = self.i32const(t);
                let st = self.rt_status(sym::DROP, &[addr, idv])?;
                let abort = self.b.create_block();
                let next = self.b.create_block();
                self.b.ins().brif(st, abort, &[], next, &[]);
                self.b.switch_to_block(abort);
                self.rt(sym::ABORT_UNWIND, &[])?;
                self.b.ins().trap(ir::TrapCode::unwrap_user(1));
                self.b.switch_to_block(next);
            }
            let fail = self.fail_block();
            self.b.ins().jump(fail, &[]);
            self.b.switch_to_block(cont);
        }
        for (addr, t) in dropped {
            let idv = self.i32const(t);
            let st = self.rt_status(sym::DROP, &[addr, idv])?;
            self.check(st);
        }
        let (daddr, dty) = self.place(dest, None)?;
        let dsize = self.size(&dty)?.min(rsize);
        self.copy_bytes(daddr, ret, dsize);
        Ok(())
    }

    fn call_intrinsic(
        &mut self,
        dest: &Place,
        name: &str,
        targs: &[Type],
        args: &[Operand],
        derefs: usize,
    ) -> Result<(), Error> {
        if let Some((kind, op)) = sized_op(name) {
            return self.call_sized(dest, kind, op, args);
        }
        if let Some((kind, op)) = wide_op(name) {
            return self.call_wide(dest, kind, op, args);
        }
        // A foreign function is called as C calls it, and the machine
        // intrinsics of std.rt are the backend's (decision 302).
        if let Some(f) = name.strip_prefix("extern.") {
            return self.call_foreign(dest, f, args);
        }
        if name.starts_with("rt.") {
            return self.machine_intrinsic(dest, name, args);
        }
        // A place of the program holds a value as it is laid out, and a
        // reference into raw memory is read so: a pointer to a place, and a
        // reference made from a pointer, are only made to a type that raw
        // memory holds as a value is laid out (decision 302).
        let pointee = match name {
            "mem.ptr.as_ref" | "mem.ptr.as_mut" | "mem.slice.as_ptr" => self.raw_pointee(args.first())?,
            "mem.ptr.of" | "mem.ptr.of_mut" => match args.first() {
                Some(Operand::Copy(p) | Operand::Move(p)) => match self.place_type(p)? {
                    Type::Ref(inner, _) => Some(*inner),
                    t => Some(t),
                },
                _ => None,
            },
            _ => None,
        };
        if let Some(t) = pointee
            && !self.cg.types.same_in_memory(&t)
        {
            return Err(Error::unsupported(format!(
                "`{name}` of a `{}`, which raw memory holds otherwise than a value of it",
                sir::write::ty(&t)
            )));
        }
        let Some(i) = Intrinsic::from_name(name) else {
            return Err(Error::unsupported(format!("the intrinsic `{name}`")));
        };
        self.cg.note_intrinsic(name, targs)?;
        let dest_ty = self.place_type(dest)?;
        let argv = self.arguments(args, derefs, None)?;
        // What the intrinsic keeps (its `own` arguments) holds counted
        // copies of capability handles; it only reads the others (decision
        // 237).
        if let Some(decl) = self.cg.prog.intrinsic(name) {
            let own = decl.own.clone();
            for k in own {
                if let (Some(Operand::Copy(_)), Some((a, t))) = (args.get(k as usize), argv.get(k as usize)) {
                    let (a, t) = (*a, t.clone());
                    self.count_at(a, &t)?;
                }
            }
        }
        // The commonest intrinsics run inline, the runtime called only
        // where it raises a panic or keeps state of its own (decision 253).
        let fast = match self.frame_iterator(i, dest, &argv)? {
            Fast::No => self.inline_intrinsic(i, dest, &argv)?,
            framed => framed,
        };
        let done = match fast {
            Fast::Done => return Ok(()),
            Fast::Slow(done) => Some(done),
            Fast::No => None,
        };
        if i.may_wait() {
            self.record_site();
        }
        let n = argv.len() as u32;
        let mut desc = vec![i as u32, n, targs.len() as u32, self.type_id(&dest_ty)?];
        for (_, t) in &argv {
            desc.push(self.type_id(t)?);
        }
        for t in targs {
            desc.push(self.type_id(t)?);
        }
        let did = self.cg.desc(desc)?;
        let daddr = self.data_addr(did);
        let arr = self.slot(8 * n);
        for (k, (a, _)) in argv.iter().enumerate() {
            self.store_word(*a, arr, (k * 8) as u32);
        }
        let rsize = self.size(&dest_ty)?;
        let ret = self.slot(rsize);
        let st = self.rt_status(sym::INTRINSIC, &[daddr, ret, arr])?;
        // A value moved into a parameter the intrinsic neither owns nor
        // lets its result hold was only read: it is dropped now, when
        // nothing observes its drop, as the interpreter's host drops it
        // (decision 218), also when the call panicked. Such a drop only
        // frees memory, so it cannot panic itself.
        if let Some(decl) = self.cg.prog.intrinsic(name) {
            let held: Vec<u32> =
                [&decl.own, &decl.from, &decl.inner, &decl.keeps].into_iter().flatten().copied().collect();
            for (k, (o, (a, t))) in args.iter().zip(&argv).enumerate() {
                let Operand::Move(p) = o else { continue };
                if held.contains(&(k as u32))
                    || i.takes_iterable(k)
                    || self.place_type(p)? != *t
                    || !self.cg.types.needs_drop(t)?
                    || self.cg.types.observable_drop(t)
                {
                    continue;
                }
                let id = self.type_id(t)?;
                let idv = self.i32const(id);
                self.rt_status(sym::DROP, &[*a, idv])?;
            }
        }
        self.check(st);
        let (addr, _) = self.place(dest, None)?;
        self.copy_bytes(addr, ret, rsize);
        if let Some(done) = done {
            self.b.ins().jump(done, &[]);
            self.b.switch_to_block(done);
        }
        Ok(())
    }

    /// The type a raw pointer operand (or a slice operand, through
    /// references) holds values of.
    fn raw_pointee(&mut self, o: Option<&Operand>) -> Result<Option<Type>, Error> {
        let Some(Operand::Copy(p) | Operand::Move(p)) = o else { return Ok(None) };
        let mut t = self.place_type(p)?;
        while let Type::Ref(inner, _) = t {
            t = *inner;
        }
        Ok(match t {
            Type::Named(n, args) if matches!(n.as_str(), "ptr" | "ptr_mut" | "Slice" | "list") => {
                args.first().cloned()
            }
            _ => None,
        })
    }

    /// A call of a foreign function (`extern.LIBRARY.NAME`), as C calls it:
    /// each argument converted to its C type, the result back to its Sisu
    /// one, and what the program printed so far written first, since the
    /// function may write to the same streams (decision 302).
    fn call_foreign(&mut self, dest: &Place, f: &str, args: &[Operand]) -> Result<(), Error> {
        let symbol = f.split_once('.').map_or(f, |(_, n)| n);
        let argv = self.arguments(args, 0, None)?;
        let mut params = Vec::with_capacity(argv.len());
        let mut vals = Vec::with_capacity(argv.len());
        for (addr, t) in &argv {
            let (ct, v) = self.c_argument(*addr, t, symbol)?;
            params.push(ct);
            vals.push(v);
        }
        let dest_ty = self.place_type(dest)?;
        let ret = match &dest_ty {
            Type::Unit => None,
            t => Some(c_type(t).ok_or_else(|| {
                Error::unsupported(format!(
                    "the result of the foreign function `{symbol}`, a `{}`",
                    sir::write::ty(t)
                ))
            })?),
        };
        // What the program printed goes out before foreign code runs; the
        // runtime in Sisu's calls are the runtime's own (decision 350).
        if !self.cg.library {
            self.rt(sym::FLUSH, &[])?;
        }
        // The debug profile checks the calls of the C functions the
        // interpreter emulates on the memory model (decision 561): the
        // runtime's form of each is called, and its status after.
        let words = params.iter().chain(&ret).all(|t| *t == types::I64);
        let checked = (!self.cg.release && !self.cg.library && words)
            .then(|| abi::checked_c(symbol, params.len(), usize::from(ret.is_some())))
            .flatten();
        let id = self.cg.c_function(checked.as_deref().unwrap_or(symbol), &params, ret.as_slice())?;
        let r = self.func_ref(id);
        let call = self.b.ins().call(r, &vals);
        let result = self.b.inst_results(call).first().copied();
        if checked.is_some() {
            let st = self.rt_status(sym::C_CHECKED, &[])?;
            self.check(st);
        }
        let (Some(v), Some(ct)) = (result, ret) else { return Ok(()) };
        let (d, _) = self.place(dest, None)?;
        let word = match &dest_ty {
            Type::Named(n, _) if n == "Option" => {
                let none = self.b.ins().icmp_imm(IntCC::Equal, v, 0);
                let tag = self.b.ins().uextend(types::I64, none);
                self.store_word(tag, d, 0);
                self.store_word(v, d, 8);
                return Ok(());
            }
            Type::Named(n, _) if n == "f32" => {
                let bits = self.b.ins().bitcast(types::I32, MemFlags::new(), v);
                self.b.ins().uextend(types::I64, bits)
            }
            Type::Float => self.b.ins().bitcast(types::I64, MemFlags::new(), v),
            Type::Named(n, _) if matches!(n.as_str(), "i8" | "i16" | "i32") => {
                self.b.ins().sextend(types::I64, v)
            }
            _ if ct != types::I64 => self.b.ins().uextend(types::I64, v),
            _ => v,
        };
        self.store_word(word, d, 0);
        Ok(())
    }

    /// An argument of a foreign call as its C type: its value loaded from
    /// `addr` and narrowed, an option of a raw pointer as a nullable one.
    fn c_argument(&mut self, addr: Value, t: &Type, symbol: &str) -> Result<(ir::Type, Value), Error> {
        let ct = c_type(t).ok_or_else(|| {
            Error::unsupported(format!(
                "an argument of the foreign function `{symbol}` of type `{}`",
                sir::write::ty(t)
            ))
        })?;
        let word = self.load(addr, 0);
        let v = match t {
            Type::Named(n, _) if n == "Option" => {
                let payload = self.load(addr, 8);
                let some = self.b.ins().icmp_imm(IntCC::Equal, word, 0);
                let zero = self.iconst(0);
                self.b.ins().select(some, payload, zero)
            }
            Type::Float => self.b.ins().bitcast(types::F64, MemFlags::new(), word),
            Type::Named(n, _) if n == "f32" => {
                let low = self.b.ins().ireduce(types::I32, word);
                self.b.ins().bitcast(types::F32, MemFlags::new(), low)
            }
            _ if ct != types::I64 => self.b.ins().ireduce(ct, word),
            _ => word,
        };
        Ok((ct, v))
    }

    /// The machine intrinsics of std.rt (22, "Machine intrinsics"; D-086):
    /// the stack pointer read in place, the thread's slot and the switch
    /// between stacks through the runtime.
    fn machine_intrinsic(&mut self, dest: &Place, name: &str, args: &[Operand]) -> Result<(), Error> {
        match name {
            "rt.stack_pointer" => {
                let sp = self.b.ins().get_stack_pointer(types::I64);
                let (d, _) = self.place(dest, None)?;
                self.store_word(sp, d, 0);
            }
            "rt.thread_slot" => {
                let slot =
                    self.rt(sym::THREAD_SLOT, &[])?.ok_or_else(|| Error::internal("no thread slot"))?;
                let (d, _) = self.place(dest, None)?;
                self.store_word(slot, d, 0);
            }
            "rt.switch" => {
                let argv = self.arguments(args, 0, None)?;
                let [(save, _), (load, _)] = argv.as_slice() else {
                    return Err(Error::internal("`rt.switch` without its two contexts"));
                };
                let (save, load) = (self.load(*save, 0), self.load(*load, 0));
                self.rt(sym::SWITCH, &[save, load])?;
            }
            _ => return Err(Error::unsupported(format!("the machine intrinsic `{name}`"))),
        }
        Ok(())
    }

    /// The slot of a local whose iterator lives in the frame.
    fn frame_of(&self, l: sir::LocalId) -> Option<ir::StackSlot> {
        self.frames.get(l.0 as usize).copied().flatten()
    }

    /// A range, or an iterator over a borrowed list's elements, made into
    /// a local whose object lives in the frame ([`frame_iterators`]): the
    /// header [`Cx::inline_intrinsic`] steps, in the local's slot, which
    /// the local's word points to. A step of zero is the runtime's, which
    /// raises the panic.
    fn frame_iterator(&mut self, i: Intrinsic, dest: &Place, argv: &[(Value, Type)]) -> Result<Fast, Error> {
        use Intrinsic as I;
        let Some(slot) = dest.proj.is_empty().then(|| self.frame_of(dest.local)).flatten() else {
            return Ok(Fast::No);
        };
        let word = |cx: &mut Self, k: usize| -> Result<Value, Error> {
            let (a, t) = argv.get(k).cloned().ok_or_else(|| Error::internal("a range without its bounds"))?;
            let (a, _) = cx.base_of(a, &t);
            Ok(cx.load(a, 0))
        };
        match i {
            I::Range => {
                let (start, stop, step) = (word(self, 0)?, word(self, 1)?, word(self, 2)?);
                let (fast, slow, done) =
                    (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(step, fast, &[], slow, &[]);
                self.b.switch_to_block(fast);
                let obj = self.b.ins().stack_addr(types::I64, slot, 0);
                let kind = self.iconst(abi::ITER_RANGE as i64);
                self.store_word(kind, obj, abi::ITER_KIND);
                self.store_word(start, obj, abi::ITER_A);
                self.store_word(stop, obj, abi::ITER_B);
                self.store_word(step, obj, abi::ITER_C);
                let (d, _) = self.place(dest, None)?;
                self.store_word(obj, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(slow);
                Ok(Fast::Slow(done))
            }
            I::IterRef | I::IterMut | I::ListIter | I::SliceIter => {
                let Some((a0, t0)) = argv.first().cloned() else { return Ok(Fast::No) };
                let (list, lt) = self.base_of(a0, &t0);
                let elem = element_type(&lt)?;
                let size = self.size(&elem)?;
                let obj = self.b.ins().stack_addr(types::I64, slot, 0);
                let kind = self.iconst(abi::ITER_LIST_REF as i64);
                let (zero, size) = (self.iconst(0), self.iconst(i64::from(size)));
                self.store_word(kind, obj, abi::ITER_KIND);
                self.store_word(list, obj, abi::ITER_A);
                self.store_word(zero, obj, abi::ITER_B);
                self.store_word(size, obj, abi::ITER_C);
                let (d, _) = self.place(dest, None)?;
                self.store_word(obj, d, 0);
                Ok(Fast::Done)
            }
            _ => Err(Error::internal(format!("an iterator in the frame made by `{}`", i.name()))),
        }
    }

    /// What a reference argument refers to, through every reference, and
    /// its type.
    fn base_of(&mut self, mut addr: Value, ty: &Type) -> (Value, Type) {
        let mut ty = ty.clone();
        while let Type::Ref(inner, _) = ty {
            addr = self.load(addr, 0);
            ty = *inner;
        }
        (addr, ty)
    }

    /// The commonest intrinsics compiled inline (decision 253), as the
    /// runtime runs them on the values' layouts: the length and the
    /// elements of a list, the bytes of a text, a character's code, an
    /// option's tag, and the next item of a range or of a borrowed list's
    /// elements. Each reads only what is borrowed or `Copy`, so nothing is
    /// left to drop. Where the runtime raises a panic (an index out of
    /// bounds) or keeps an iterator's state itself, the code calls it
    /// after all; where it grows a list, builds a text or looks a key up,
    /// the code calls an entry of its own, without the descriptor
    /// (decision 265).
    /// Raw memory of a word or a byte without the runtime (decision 335): a
    /// raw pointer's address, made, cast or offset, and the value at it read
    /// or written at its width, for the types whose raw memory is their
    /// value's word and for `u8` and `i8`, a byte; `None` for the others,
    /// which the runtime converts part by part (decision 302). The debug
    /// profile offsets, reads and writes through the runtime, which checks
    /// them (decision 543).
    fn raw_inline(
        &mut self,
        i: Intrinsic,
        dest: &Place,
        b: Value,
        bt: &Type,
        argv: &[(Value, Type)],
    ) -> Result<Option<Fast>, Error> {
        use Intrinsic as I;
        let pointer = matches!(bt, Type::Named(n, _) if n == "ptr" || n == "ptr_mut");
        let word = match i {
            I::MemPtrAddr | I::MemPtrCast | I::MemPtrCastMut | I::MemPtrAsConst if pointer => true,
            I::MemPtrFromAddress => *bt == Type::Int,
            _ => false,
        };
        if word {
            let w = self.load(b, 0);
            let (d, _) = self.place(dest, None)?;
            self.store_word(w, d, 0);
            return Ok(Some(Fast::Done));
        }
        let scalar = match bt {
            Type::Named(_, args) if pointer => args.first().and_then(raw_scalar),
            _ => None,
        };
        let Some((width, signed)) = scalar else { return Ok(None) };
        // The debug profile checks raw memory (decision 543): the runtime
        // offsets, reads and writes, but in the runtime's own code.
        if !self.cg.release && !self.cg.library {
            return Ok(None);
        }
        match i {
            I::MemPtrOffset => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(None) };
                let (na, _) = self.base_of(a1, &t1);
                let n = self.load(na, 0);
                let p = self.load(b, 0);
                let off = self.b.ins().imul_imm(n, i64::from(width));
                let q = self.b.ins().iadd(p, off);
                let (d, _) = self.place(dest, None)?;
                self.store_word(q, d, 0);
                Ok(Some(Fast::Done))
            }
            I::MemPtrRead => {
                let p = self.load(b, 0);
                let v = match (width, signed) {
                    (1, false) => self.b.ins().uload8(types::I64, MemFlags::trusted(), p, 0),
                    (1, true) => self.b.ins().sload8(types::I64, MemFlags::trusted(), p, 0),
                    _ => self.load(p, 0),
                };
                let (d, _) = self.place(dest, None)?;
                self.store_word(v, d, 0);
                Ok(Some(Fast::Done))
            }
            I::MemPtrWrite => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(None) };
                let (va, _) = self.base_of(a1, &t1);
                let v = self.load(va, 0);
                let p = self.load(b, 0);
                if width == 1 {
                    let byte = self.b.ins().ireduce(types::I8, v);
                    self.store_word(byte, p, 0);
                } else {
                    self.store_word(v, p, 0);
                }
                Ok(Some(Fast::Done))
            }
            _ => Ok(None),
        }
    }

    fn inline_intrinsic(
        &mut self,
        i: Intrinsic,
        dest: &Place,
        argv: &[(Value, Type)],
    ) -> Result<Fast, Error> {
        use Intrinsic as I;
        let named = |t: &Type, names: &[&str]| matches!(t, Type::Named(n, _) if names.contains(&n.as_str()));
        let text = |t: &Type| matches!(t, Type::Str) || named(t, &["String", "Path"]);
        // A formatter is the `String` it builds, empty at first (decision
        // 265).
        if i == I::FmtNew && named(&self.place_type(dest)?, &["String"]) {
            let (d, _) = self.place(dest, None)?;
            let zero = self.iconst(0);
            for k in 0..3 {
                self.store_word(zero, d, 8 * k);
            }
            return Ok(Fast::Done);
        }
        let Some((a0, t0)) = argv.first().cloned() else { return Ok(Fast::No) };
        // Only arguments read in place, or holding nothing to drop: a value
        // moved into the call may be the runtime's to drop afterwards
        // (decision 218).
        let borrowed = matches!(t0, Type::Ref(..) | Type::Str) || !self.cg.types.needs_drop(&t0)?;
        if !borrowed {
            return Ok(Fast::No);
        }
        let (b, bt) = self.base_of(a0, &t0);
        if let Some(done) = self.raw_inline(i, dest, b, &bt, argv)? {
            return Ok(done);
        }
        match i {
            I::Len if named(&bt, &["list", "bytes"]) => {
                let len = self.load(b, 8);
                let (d, _) = self.place(dest, None)?;
                self.store_word(len, d, 0);
                Ok(Fast::Done)
            }
            // A reference to a `Shared`'s value is the address the handle
            // holds; reaching the cell counts for `--stats`, as a
            // dereference does (decision 260).
            I::SharedGet if named(&bt, &["Shared"]) => {
                let v = self.load(b, 0);
                self.rt(sym::SHARED_TOUCH, &[v])?;
                let (d, _) = self.place(dest, None)?;
                self.store_word(v, d, 0);
                Ok(Fast::Done)
            }
            // Text appended to a `String` (a formatter's too), and a new
            // `String` from a text, by the runtime's own entries rather
            // than the descriptor (decision 265): the text's address and
            // length, or an `int`'s value for its digits. Only what is read
            // in place: a value moved in is the runtime's to drop afterwards
            // (decision 218).
            I::FmtText | I::FmtDisplay | I::StringPush if named(&bt, &["String"]) => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                if !matches!(t1, Type::Ref(..) | Type::Str) && self.cg.types.needs_drop(&t1)? {
                    return Ok(Fast::No);
                }
                let (v, vt) = self.base_of(a1, &t1);
                // The runtime in Sisu's (decision 350).
                if text(&vt) {
                    let (ptr, len) = (self.load(v, 0), self.load(v, 8));
                    self.rt_sisu(sym::TEXT_PUSH, &[b, ptr, len])?;
                    return Ok(Fast::Done);
                }
                if vt == Type::Int && i == I::FmtDisplay {
                    let n = self.load(v, 0);
                    self.rt_sisu(sym::TEXT_PUSH_INT, &[b, n])?;
                    return Ok(Fast::Done);
                }
                Ok(Fast::No)
            }
            I::StrToString | I::StringNew if text(&bt) && named(&self.place_type(dest)?, &["String"]) => {
                let (ptr, len) = (self.load(b, 0), self.load(b, 8));
                let (d, _) = self.place(dest, None)?;
                self.rt_sisu(sym::TEXT_FROM, &[d, ptr, len])?;
                Ok(Fast::Done)
            }
            // A key looked up in a dict or a set by the runtime's own entry
            // rather than the descriptor (decision 265): the address of its
            // value, or whether it is there. The key is read in place; a
            // `str` looked up among `String` keys (decision 784) is hashed
            // and compared as one of them (decision 861).
            I::DictGet | I::DictContains | I::SetContains | I::DictGetStr | I::SetContainsStr
                if named(&bt, &["dict", "set"]) =>
            {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                if !matches!(t1, Type::Ref(..) | Type::Str) && self.cg.types.needs_drop(&t1)? {
                    return Ok(Fast::No);
                }
                let kt = self.type_id(&t1)?;
                let kt = self.i32const(kt);
                let get = i == I::DictGet || i == I::DictGetStr;
                let value = self.i32const(u32::from(get));
                let found = self.rt_status(sym::TABLE_FIND, &[b, a1, kt, value])?;
                let (d, _) = self.place(dest, None)?;
                if get {
                    // `Some` (tag 0) of the value's address, or `None` (1).
                    let none = self.b.ins().icmp_imm(IntCC::Equal, found, 0);
                    let tag = self.b.ins().uextend(types::I64, none);
                    self.store_word(tag, d, 0);
                    self.store_word(found, d, 8);
                } else {
                    self.store_word(found, d, 0);
                }
                Ok(Fast::Done)
            }
            // `d[k]` of a `str` among `String` keys (decision 861): the
            // value's address from `d{k}`'s slot, whose read of a missing key
            // panics naming it.
            I::DictIndexStr if named(&bt, &["dict"]) => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                if !matches!(t1, Type::Ref(..) | Type::Str) && self.cg.types.needs_drop(&t1)? {
                    return Ok(Fast::No);
                }
                let kt = self.type_id(&t1)?;
                let (kv, mv) = (self.i32const(kt), self.i32const(abi::SLOT_READ));
                let out = self.slot(8);
                let st = self.rt_status(sym::DICT_SLOT, &[b, a1, kv, mv, out])?;
                self.check(st);
                let at = self.load(out, 0);
                let (d, _) = self.place(dest, None)?;
                self.store_word(at, d, 0);
                Ok(Fast::Done)
            }
            // A `str`'s key taken out of a dict or a set of `String` keys
            // (decision 861): the runtime's entry writes the option of the
            // value, or whether it was there, and drops the key it held.
            I::DictRemoveStr | I::SetRemoveStr if named(&bt, &["dict", "set"]) => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                if !matches!(t1, Type::Ref(..) | Type::Str) && self.cg.types.needs_drop(&t1)? {
                    return Ok(Fast::No);
                }
                let kt = self.type_id(&t1)?;
                let kv = self.i32const(kt);
                let value = self.i32const(u32::from(i == I::DictRemoveStr));
                let (d, _) = self.place(dest, None)?;
                let st = self.rt_status(sym::TABLE_TAKE, &[b, a1, kv, value, d])?;
                self.check(st);
                Ok(Fast::Done)
            }
            I::ListIsEmpty if named(&bt, &["list"]) => {
                let len = self.load(b, 8);
                let empty = self.b.ins().icmp_imm(IntCC::Equal, len, 0);
                let w = self.b.ins().uextend(types::I64, empty);
                let (d, _) = self.place(dest, None)?;
                self.store_word(w, d, 0);
                Ok(Fast::Done)
            }
            I::StrByteLen if text(&bt) => {
                let len = self.load(b, 8);
                let (d, _) = self.place(dest, None)?;
                self.store_word(len, d, 0);
                Ok(Fast::Done)
            }
            I::CharCode if matches!(bt, Type::Char) => {
                let c = self.load(b, 0);
                let (d, _) = self.place(dest, None)?;
                self.store_word(c, d, 0);
                Ok(Fast::Done)
            }
            I::OptionIsSome | I::OptionIsNone if named(&bt, &["Option"]) => {
                let tag = self.load(b, 0);
                let cc = if i == I::OptionIsSome { IntCC::Equal } else { IntCC::NotEqual };
                let is = self.b.ins().icmp_imm(cc, tag, 0);
                let w = self.b.ins().uextend(types::I64, is);
                let (d, _) = self.place(dest, None)?;
                self.store_word(w, d, 0);
                Ok(Fast::Done)
            }
            I::ListFirst | I::ListLast if named(&bt, &["list"]) => {
                let elem = element_type(&bt)?;
                let size = self.size(&elem)?;
                let len = self.load(b, 8);
                let data = self.load(b, 0);
                let (d, _) = self.place(dest, None)?;
                let (some, none, done) =
                    (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(len, some, &[], none, &[]);
                self.b.switch_to_block(some);
                let at = if i == I::ListFirst { self.iconst(0) } else { self.b.ins().iadd_imm(len, -1) };
                let off = self.b.ins().imul_imm(at, i64::from(size));
                let item = self.b.ins().iadd(data, off);
                let zero = self.iconst(0);
                self.store_word(zero, d, 0);
                self.store_word(item, d, 8);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(none);
                let one = self.iconst(1);
                self.store_word(one, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(done);
                Ok(Fast::Done)
            }
            I::ListPop if named(&bt, &["list"]) => {
                let elem = element_type(&bt)?;
                let size = self.size(&elem)?;
                let len = self.load(b, 8);
                let (d, _) = self.place(dest, None)?;
                let (some, none, done) =
                    (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(len, some, &[], none, &[]);
                self.b.switch_to_block(some);
                let last = self.b.ins().iadd_imm(len, -1);
                self.store_word(last, b, 8);
                let data = self.load(b, 0);
                let off = self.b.ins().imul_imm(last, i64::from(size));
                let src = self.b.ins().iadd(data, off);
                let zero = self.iconst(0);
                self.store_word(zero, d, 0);
                let payload = self.b.ins().iadd_imm(d, 8);
                self.copy_bytes(payload, src, size);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(none);
                let one = self.iconst(1);
                self.store_word(one, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(done);
                Ok(Fast::Done)
            }
            I::StrByteAt if text(&bt) => {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                let (ia, _) = self.base_of(a1, &t1);
                let idx = self.load(ia, 0);
                let len = self.load(b, 8);
                let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, idx, len);
                let (d, _) = self.place(dest, None)?;
                let (fast, slow, done) =
                    (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(inside, fast, &[], slow, &[]);
                self.b.switch_to_block(fast);
                let data = self.load(b, 0);
                let at = self.b.ins().iadd(data, idx);
                let byte = self.b.ins().uload8(types::I64, MemFlags::trusted(), at, 0);
                self.store_word(byte, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(slow);
                Ok(Fast::Slow(done))
            }
            I::StrByteSlice if text(&bt) => {
                let (Some((a1, t1)), Some((a2, t2))) = (argv.get(1).cloned(), argv.get(2).cloned()) else {
                    return Ok(Fast::No);
                };
                let (p1, _) = self.base_of(a1, &t1);
                let (p2, _) = self.base_of(a2, &t2);
                let (start, end) = (self.load(p1, 0), self.load(p2, 0));
                let (data, len) = (self.load(b, 0), self.load(b, 8));
                // `0 <= start <= end <= len`, as unsigned comparisons.
                let x = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, start, end);
                let y = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, len);
                let inside = self.b.ins().band(x, y);
                let (bounds, fast, slow, done) = (
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                    self.b.create_block(),
                );
                self.b.ins().brif(inside, bounds, &[], slow, &[]);
                self.b.switch_to_block(bounds);
                // Each end at a character's first byte, or at either end.
                let at_char = |cx: &mut Self, k: Value| {
                    let before = cx.b.ins().icmp(IntCC::UnsignedLessThan, k, len);
                    let byte = cx.b.ins().iadd(data, k);
                    // A byte past the text is never read: the text's own
                    // header stands in for it.
                    let at = cx.b.ins().select(before, byte, b);
                    let first = cx.b.ins().sload8(types::I64, MemFlags::trusted(), at, 0);
                    let lead = cx.b.ins().icmp_imm(IntCC::SignedGreaterThanOrEqual, first, -0x40);
                    let edge = cx.b.ins().icmp_imm(IntCC::Equal, k, 0);
                    let end = cx.b.ins().icmp(IntCC::Equal, k, len);
                    let either = cx.b.ins().bor(edge, end);
                    cx.b.ins().bor(lead, either)
                };
                let (x, y) = (at_char(self, start), at_char(self, end));
                let both = self.b.ins().band(x, y);
                self.b.ins().brif(both, fast, &[], slow, &[]);
                self.b.switch_to_block(fast);
                let from = self.b.ins().iadd(data, start);
                let n = self.b.ins().isub(end, start);
                let (d, _) = self.place(dest, None)?;
                self.store_word(from, d, 0);
                self.store_word(n, d, 8);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(slow);
                Ok(Fast::Slow(done))
            }
            I::CharFromCode if matches!(bt, Type::Int) => {
                // A scalar value: at most U+10FFFF, not a surrogate.
                let v = self.load(b, 0);
                let below = self.b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, v, 0x10FFFF);
                let off = self.b.ins().iadd_imm(v, -0xD800);
                let surrogate = self.b.ins().icmp_imm(IntCC::UnsignedLessThan, off, 0x800);
                let valid = self.b.ins().band_not(below, surrogate);
                let (some, none) = (self.iconst(0), self.iconst(1));
                let tag = self.b.ins().select(valid, some, none);
                let (d, _) = self.place(dest, None)?;
                self.store_word(tag, d, 0);
                self.store_word(v, d, 8);
                Ok(Fast::Done)
            }
            I::CharIsWhitespace | I::CharIsAlphabetic if matches!(bt, Type::Char) => {
                // An ASCII character inline, any other the runtime's.
                let c = self.load(b, 0);
                let ascii = self.b.ins().icmp_imm(IntCC::UnsignedLessThan, c, 0x80);
                let (fast, slow, done) =
                    (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(ascii, fast, &[], slow, &[]);
                self.b.switch_to_block(fast);
                let is = if i == I::CharIsWhitespace {
                    // A space, or `\t`, `\n`, `\v`, `\f` and `\r`.
                    let space = self.b.ins().icmp_imm(IntCC::Equal, c, 0x20);
                    let k = self.b.ins().iadd_imm(c, -0x09);
                    let control = self.b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, k, 4);
                    self.b.ins().bor(space, control)
                } else {
                    let lower = self.b.ins().bor_imm(c, 0x20);
                    let k = self.b.ins().iadd_imm(lower, -i64::from(b'a'));
                    self.b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, k, 25)
                };
                let w = self.b.ins().uextend(types::I64, is);
                let (d, _) = self.place(dest, None)?;
                self.store_word(w, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(slow);
                Ok(Fast::Slow(done))
            }
            // Two `int`s, or references to them, which a generic function
            // that takes its `T` by reference passes (decision 59): the
            // result is the value either way (decision 264).
            I::Max | I::Min
                if bt == Type::Int
                    && argv.get(1).is_some_and(|(_, t)| target_type(t) == Type::Int)
                    && self.place_type(dest)? == Type::Int =>
            {
                let Some((a1, t1)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                let (b1, _) = self.base_of(a1, &t1);
                let (x, y) = (self.load(b, 0), self.load(b1, 0));
                // The first unless the second is the greater (`max`) or
                // the less (`min`).
                let cc = if i == I::Max { IntCC::SignedLessThan } else { IntCC::SignedGreaterThan };
                let second = self.b.ins().icmp(cc, x, y);
                let r = self.b.ins().select(second, y, x);
                let (d, _) = self.place(dest, None)?;
                self.store_word(r, d, 0);
                Ok(Fast::Done)
            }
            I::ListPush if named(&bt, &["list"]) => {
                let Some((a1, _)) = argv.get(1).cloned() else { return Ok(Fast::No) };
                let elem = element_type(&bt)?;
                let size = self.size(&elem)?;
                let len = self.load(b, 8);
                let cap = self.load(b, 16);
                let room = self.b.ins().icmp(IntCC::UnsignedLessThan, len, cap);
                let (fast, grow) = (self.b.create_block(), self.b.create_block());
                self.b.ins().brif(room, fast, &[], grow, &[]);
                // Without room, the runtime grows the buffer as its push
                // would, and the element goes in as with room (decision 265).
                self.b.switch_to_block(grow);
                let bytes = self.iconst(i64::from(size));
                self.rt(sym::LIST_RESERVE, &[b, bytes])?;
                self.b.ins().jump(fast, &[]);
                self.b.switch_to_block(fast);
                let data = self.load(b, 0);
                let off = self.b.ins().imul_imm(len, i64::from(size));
                let at = self.b.ins().iadd(data, off);
                self.copy_bytes(at, a1, size);
                let more = self.b.ins().iadd_imm(len, 1);
                self.store_word(more, b, 8);
                Ok(Fast::Done)
            }
            I::IterNext if named(&bt, &["Iter"]) => {
                let obj = self.load(b, 0);
                let (d, _) = self.place(dest, None)?;
                let blocks: Vec<Block> = (0..8).map(|_| self.b.create_block()).collect();
                let (null_ok, range, not_range, more, list, list_some, none, slow) =
                    (blocks[0], blocks[1], blocks[2], blocks[3], blocks[4], blocks[5], blocks[6], blocks[7]);
                let done = self.b.create_block();
                // A null iterator (one moved out) is the runtime's.
                self.b.ins().brif(obj, null_ok, &[], slow, &[]);
                self.b.switch_to_block(null_ok);
                let kind = self.load(obj, abi::ITER_KIND);
                let is_range = self.b.ins().icmp_imm(IntCC::Equal, kind, abi::ITER_RANGE as i64);
                self.b.ins().brif(is_range, range, &[], not_range, &[]);
                // A range: the current value, unless the step past it
                // overflows, which the runtime turns into its end.
                self.b.switch_to_block(range);
                let cur = self.load(obj, abi::ITER_A);
                let stop = self.load(obj, abi::ITER_B);
                let step = self.load(obj, abi::ITER_C);
                let up = self.b.ins().icmp_imm(IntCC::SignedGreaterThan, step, 0);
                let below = self.b.ins().icmp(IntCC::SignedLessThan, cur, stop);
                let above = self.b.ins().icmp(IntCC::SignedGreaterThan, cur, stop);
                let left = self.b.ins().select(up, below, above);
                self.b.ins().brif(left, more, &[], none, &[]);
                self.b.switch_to_block(more);
                let next = self.b.ins().iadd(cur, step);
                // Overflow: the sum's sign differs from both operands'.
                let x = self.b.ins().bxor(cur, next);
                let y = self.b.ins().bxor(step, next);
                let both = self.b.ins().band(x, y);
                let over = self.b.ins().icmp_imm(IntCC::SignedLessThan, both, 0);
                // Past the last value the type holds, the range ends: its
                // current value becomes its end, as the runtime's ends it.
                let next = self.b.ins().select(over, stop, next);
                self.store_word(next, obj, abi::ITER_A);
                let zero = self.iconst(0);
                self.store_word(zero, d, 0);
                self.store_word(cur, d, 8);
                self.b.ins().jump(done, &[]);
                // A borrowed list's elements: the address of the next one.
                self.b.switch_to_block(not_range);
                let is_list = self.b.ins().icmp_imm(IntCC::Equal, kind, abi::ITER_LIST_REF as i64);
                self.b.ins().brif(is_list, list, &[], slow, &[]);
                self.b.switch_to_block(list);
                let l = self.load(obj, abi::ITER_A);
                let idx = self.load(obj, abi::ITER_B);
                let len = self.load(l, 8);
                let has = self.b.ins().icmp(IntCC::UnsignedLessThan, idx, len);
                self.b.ins().brif(has, list_some, &[], none, &[]);
                self.b.switch_to_block(list_some);
                let data = self.load(l, 0);
                let size = self.load(obj, abi::ITER_C);
                let off = self.b.ins().imul(idx, size);
                let item = self.b.ins().iadd(data, off);
                let next = self.b.ins().iadd_imm(idx, 1);
                self.store_word(next, obj, abi::ITER_B);
                let zero = self.iconst(0);
                self.store_word(zero, d, 0);
                self.store_word(item, d, 8);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(none);
                let one = self.iconst(1);
                self.store_word(one, d, 0);
                self.b.ins().jump(done, &[]);
                self.b.switch_to_block(slow);
                Ok(Fast::Slow(done))
            }
            _ => Ok(Fast::No),
        }
    }
    fn call_value(&mut self, dest: &Place, f: &Operand, args: &[Operand]) -> Result<(), Error> {
        let (v, mut ty) = self.operand(f)?;
        let size = self.size(&ty)?;
        let mut addr = self.in_memory(v, size);
        while let Type::Ref(inner, _) = ty {
            ty = *inner;
            addr = self.load(addr, 0);
        }
        let Type::Fn(_, ret, _) = &ty else {
            return Err(Error::internal(format!("calling a `{}`", sir::write::ty(&ty))));
        };
        let ret_ty = (**ret).clone();
        let code = self.load(addr, 0);
        // The address of the environment the value holds (decision 224).
        let env = self.b.ins().iadd_imm(addr, i64::from(abi::WORD));
        let argv = self.arguments(args, 0, None)?;
        // The callee owns its arguments: copies of handles are counted
        // (decision 237).
        for (o, (a, t)) in args.iter().zip(&argv) {
            if !matches!(o, Operand::Move(_)) {
                self.count_at(*a, t)?;
            }
        }
        let rsize = self.size(&ret_ty)?;
        let retp = self.slot(rsize);
        let mut call_args = vec![retp, env];
        call_args.extend(argv.iter().map(|(a, _)| *a));
        let sig = crate::sisu_signature(self.cg.call_conv, argv.len() as u32 + 1);
        let sigref = self.b.import_signature(sig);
        let call = self.b.ins().call_indirect(sigref, code, &call_args);
        let st = self.b.inst_results(call)[0];
        self.check(st);
        let (daddr, dty) = self.place(dest, None)?;
        let dsize = self.size(&dty)?.min(rsize);
        self.copy_bytes(daddr, retp, dsize);
        Ok(())
    }

    /// An operation of a sized integer type (`sisu_rt_sized`).
    fn call_sized(&mut self, dest: &Place, kind: u32, op: SizedOp, args: &[Operand]) -> Result<(), Error> {
        let mut words = Vec::new();
        let mut flags = 0;
        for (k, o) in args.iter().take(2).enumerate() {
            let (w, t) = self.scalar(o)?;
            if matches!(&t, Type::Named(n, _) if n == "u64") {
                flags |= if k == 0 { 1 } else { 2 };
            }
            words.push(w);
        }
        while words.len() < 2 {
            words.push(self.iconst(0));
        }
        // The operations that can neither fail nor leave the type's range,
        // on the words as they hold the values, inline (decision 253).
        let (x, y) = (words[0], words[1]);
        let unsigned = matches!(abi::SIZED_TYPES.get(kind as usize), Some(t) if t.starts_with('u'));
        let fast = match op {
            SizedOp::ToInt if flags == 0 => Some(x),
            SizedOp::Widen => Some(x),
            SizedOp::And => Some(self.b.ins().band(x, y)),
            SizedOp::Or => Some(self.b.ins().bor(x, y)),
            SizedOp::Xor => Some(self.b.ins().bxor(x, y)),
            SizedOp::Cmp => {
                // 0, 1 or 2: less, equal or greater.
                let (lt, gt) = if flags != 0 {
                    (IntCC::UnsignedLessThan, IntCC::UnsignedGreaterThan)
                } else {
                    (IntCC::SignedLessThan, IntCC::SignedGreaterThan)
                };
                let (lt, gt) = (self.b.ins().icmp(lt, x, y), self.b.ins().icmp(gt, x, y));
                let (lt, gt) = (self.b.ins().uextend(types::I64, lt), self.b.ins().uextend(types::I64, gt));
                let d = self.b.ins().isub(gt, lt);
                Some(self.b.ins().iadd_imm(d, 1))
            }
            SizedOp::Not if !unsigned => Some(self.b.ins().bnot(x)),
            // An `int` reduced to the type's width, as two's complement
            // wraps it: masked, or its low bits extended by their sign.
            SizedOp::WrappingFrom => match abi::SIZED_TYPES.get(kind as usize).copied() {
                Some("u64") => Some(x),
                Some(t) => {
                    let bits: i64 = match t {
                        "u8" | "i8" => 8,
                        "u16" | "i16" => 16,
                        _ => 32,
                    };
                    if unsigned {
                        let mask = self.iconst((1i64 << bits) - 1);
                        Some(self.b.ins().band(x, mask))
                    } else {
                        let shift = self.iconst(64 - bits);
                        let up = self.b.ins().ishl(x, shift);
                        Some(self.b.ins().sshr(up, shift))
                    }
                }
                None => None,
            },
            _ => None,
        };
        if let Some(r) = fast {
            let (addr, _) = self.place(dest, None)?;
            self.store_word(r, addr, 0);
            return Ok(());
        }
        let out = self.slot(16);
        let (kv, ov, fv) = (self.i32const(kind), self.i32const(op as u32), self.i32const(flags));
        let st = self.rt_status(sym::SIZED, &[kv, ov, fv, x, y, out])?;
        self.check(st);
        let (addr, ty) = self.place(dest, None)?;
        let size = self.size(&ty)?;
        self.copy_bytes(addr, out, size);
        Ok(())
    }

    /// An operation of `i128` or `u128` (`sisu_rt_wide`): the operands in
    /// memory, with their types.
    fn call_wide(&mut self, dest: &Place, kind: u32, op: SizedOp, args: &[Operand]) -> Result<(), Error> {
        let mut operands = Vec::new();
        for o in args.iter().take(2) {
            let (p, t) = self.operand_in_memory(o)?;
            let id = self.type_id(&t)?;
            operands.push((p, self.i32const(id)));
        }
        while operands.len() < 2 {
            operands.push((self.iconst(0), self.i32const(0)));
        }
        // At most an option of a 128-bit integer: a tag and two words.
        let out = self.slot(24);
        let (kv, ov) = (self.i32const(kind), self.i32const(op as u32));
        let ((x, xt), (y, yt)) = (operands[0], operands[1]);
        let st = self.rt_status(sym::WIDE, &[kv, ov, x, xt, y, yt, out])?;
        self.check(st);
        let (addr, ty) = self.place(dest, None)?;
        let size = self.size(&ty)?;
        self.copy_bytes(addr, out, size);
        Ok(())
    }
}

/// Whether a local is a flag of drop elaboration's: named `flag`, with no
/// place in the source (a variable the program names `flag` has one).
/// Whether a local of type `t` is one word the release profile may keep
/// in a variable: a number, a `bool`, a `char`, a reference that is an
/// address (not a `dyn`'s two words), or a raw pointer (decision 350).
fn one_word(t: &Type) -> bool {
    match t {
        Type::Ref(inner, _) => !crate::layout::fat(inner),
        Type::Named(n, a) if (n == "ptr" || n == "ptr_mut") && a.len() == 1 => true,
        _ => matches!(
            scalar_kind(t),
            Some(Scalar::Int | Scalar::U64 | Scalar::Float | Scalar::F32 | Scalar::Bool | Scalar::Char)
        ),
    }
}

/// The locals the release profile keeps in variables (decision 246): the
/// parameters and other locals (not the result, not a closure's
/// environment) of one word ([`one_word`]) whose own memory no code needs:
/// none borrows one, projects one other than a reference through it, or
/// uses one as a key.
fn register_locals(f: &sir::Function, tys: &[Type], closure: bool) -> Vec<bool> {
    let mut u = Uses {
        ok: tys.iter().enumerate().map(|(i, t)| i >= 1 && !(closure && i == 1) && one_word(t)).collect(),
        is_ref: tys.iter().map(|t| matches!(t, Type::Ref(..))).collect(),
    };
    for b in &f.blocks {
        for st in &b.stmts {
            match &st.kind {
                StatementKind::Assign(p, r) => {
                    u.place(p);
                    match r {
                        Rvalue::Use(o) | Rvalue::Unary(_, o) => u.operand(o),
                        Rvalue::Binary(_, x, y) => {
                            u.operand(x);
                            u.operand(y);
                        }
                        Rvalue::Aggregate(_, os) | Rvalue::Closure(_, _, os) => {
                            os.iter().for_each(|o| u.operand(o))
                        }
                        // These take the place's address: the local's own
                        // memory, unless through its reference.
                        Rvalue::Ref(q, _)
                        | Rvalue::Discriminant(q)
                        | Rvalue::CopyOf(q)
                        | Rvalue::StrView(q) => {
                            if q.proj.is_empty() {
                                u.no(q.local);
                            }
                            u.place(q);
                        }
                    }
                }
                StatementKind::Call { dest, callee, args } => {
                    u.place(dest);
                    if let Callee::Value(o) = callee {
                        u.operand(o);
                    }
                    args.iter().for_each(|o| u.operand(o));
                }
                StatementKind::Drop { place, .. } => u.place(place),
                StatementKind::StorageDead(_) => {}
            }
        }
        if let Terminator::Switch { value, .. } = &b.term {
            u.operand(value);
        }
    }
    u.ok
}

/// Which locals may be variables, as [`register_locals`] finds them out.
struct Uses {
    ok: Vec<bool>,
    is_ref: Vec<bool>,
}

impl Uses {
    fn no(&mut self, l: sir::LocalId) {
        if let Some(x) = self.ok.get_mut(l.0 as usize) {
            *x = false;
        }
    }

    /// A place reaches its local's memory unless it is the local itself
    /// (read or written whole) or what a reference in it refers to.
    fn place(&mut self, p: &Place) {
        let through =
            p.proj.first() == Some(&Proj::Deref) && self.is_ref.get(p.local.0 as usize) == Some(&true);
        if !p.proj.is_empty() && !through {
            self.no(p.local);
        }
        for q in &p.proj {
            if let Proj::Key(k) = q {
                self.no(*k);
            }
        }
    }

    fn operand(&mut self, o: &Operand) {
        if let Operand::Copy(p) | Operand::Move(p) = o {
            self.place(p);
        }
    }
}

/// The locals whose iterators live in the frame (decision 253): an
/// iterator made once, by `range` or over a borrowed list's elements, that
/// nothing but `Iter.next` reads, through references made for it. It
/// never leaves the function, the runtime never sees it, and dropping it
/// frees nothing, so its object is a slot of the frame, made and stepped
/// inline.
fn frame_iterators(f: &sir::Function, tys: &[Type]) -> Vec<bool> {
    let n = tys.len();
    let iter = |t: &Type| matches!(t, Type::Named(name, _) if name == "Iter");
    let mut fr = Frames {
        ok: tys.iter().enumerate().map(|(i, t)| i > f.param_count as usize && iter(t)).collect(),
        borrow: vec![None; n],
    };
    // The references that step them: `T = ref L`, and nothing else in T.
    for b in &f.blocks {
        for st in &b.stmts {
            if let StatementKind::Assign(p, Rvalue::Ref(q, _)) = &st.kind
                && p.proj.is_empty()
                && q.proj.is_empty()
                && fr.ok.get(q.local.0 as usize) == Some(&true)
                && let Some(slot) = fr.borrow.get_mut(p.local.0 as usize)
            {
                match *slot {
                    None => *slot = Some(q.local.0 as usize),
                    Some(m) if m == q.local.0 as usize => {}
                    Some(m) => {
                        fr.no(m);
                        fr.no(q.local.0 as usize);
                    }
                }
            }
        }
    }
    let mut made = vec![0u32; n];
    let local_ty = |o: &Operand| match o {
        Operand::Copy(p) | Operand::Move(p) if p.proj.is_empty() => {
            tys.get(p.local.0 as usize).map(target_type)
        }
        _ => None,
    };
    let maker = |name: &str, args: &[Operand]| match Intrinsic::from_name(name) {
        Some(Intrinsic::Range) => true,
        Some(Intrinsic::IterRef | Intrinsic::IterMut | Intrinsic::ListIter | Intrinsic::SliceIter) => {
            matches!(args.first().and_then(local_ty), Some(Type::Named(l, _)) if l == "list")
        }
        _ => false,
    };
    for b in &f.blocks {
        for st in &b.stmts {
            match &st.kind {
                StatementKind::Assign(p, Rvalue::Ref(q, _))
                    if p.proj.is_empty()
                        && q.proj.is_empty()
                        && fr.borrow.get(p.local.0 as usize).copied().flatten()
                            == Some(q.local.0 as usize) => {}
                StatementKind::Assign(p, r) => {
                    fr.place(p);
                    match r {
                        Rvalue::Use(o) | Rvalue::Unary(_, o) => fr.operand(o),
                        Rvalue::Binary(_, x, y) => {
                            fr.operand(x);
                            fr.operand(y);
                        }
                        Rvalue::Aggregate(_, os) | Rvalue::Closure(_, _, os) => {
                            os.iter().for_each(|o| fr.operand(o))
                        }
                        Rvalue::Ref(q, _)
                        | Rvalue::Discriminant(q)
                        | Rvalue::CopyOf(q)
                        | Rvalue::StrView(q) => fr.place(q),
                    }
                }
                StatementKind::Call { dest, callee: Callee::Intrinsic(name, _), args }
                    if name == Intrinsic::IterNext.name()
                        && matches!(args.as_slice(), [Operand::Copy(p) | Operand::Move(p)]
                            if p.proj.is_empty() && fr.borrow.get(p.local.0 as usize).copied().flatten().is_some()) =>
                {
                    fr.place(dest)
                }
                StatementKind::Call { dest, callee: Callee::Intrinsic(name, _), args }
                    if dest.proj.is_empty()
                        && fr.ok.get(dest.local.0 as usize) == Some(&true)
                        && maker(name, args) =>
                {
                    if let Some(m) = made.get_mut(dest.local.0 as usize) {
                        *m += 1;
                    }
                    args.iter().for_each(|o| fr.operand(o));
                }
                StatementKind::Call { dest, callee, args } => {
                    fr.place(dest);
                    if let Callee::Value(o) = callee {
                        fr.operand(o);
                    }
                    args.iter().for_each(|o| fr.operand(o));
                }
                StatementKind::Drop { place, .. } if place.proj.is_empty() => {}
                StatementKind::Drop { place, .. } => fr.place(place),
                StatementKind::StorageDead(_) => {}
            }
        }
        if let Terminator::Switch { value, .. } = &b.term {
            fr.operand(value);
        }
    }
    fr.ok.iter().zip(&made).map(|(ok, m)| *ok && *m == 1).collect()
}

/// What [`frame_iterators`] finds out as it goes: which locals may still
/// hold an iterator in the frame, and which reference steps which.
struct Frames {
    ok: Vec<bool>,
    borrow: Vec<Option<usize>>,
}

impl Frames {
    fn no(&mut self, i: usize) {
        if let Some(x) = self.ok.get_mut(i) {
            *x = false;
        }
    }

    /// A use of a local other than the ones allowed: an iterator, or the
    /// iterator a reference steps, keeps its object on the heap.
    fn mention(&mut self, l: sir::LocalId) {
        let i = l.0 as usize;
        self.no(i);
        if let Some(m) = self.borrow.get(i).copied().flatten() {
            self.no(m);
        }
    }

    fn place(&mut self, p: &Place) {
        self.mention(p.local);
        for q in &p.proj {
            if let Proj::Index(k) | Proj::IndexFromEnd(k) | Proj::Key(k) = q {
                self.mention(*k);
            }
        }
    }

    fn operand(&mut self, o: &Operand) {
        if let Operand::Copy(p) | Operand::Move(p) = o {
            self.place(p);
        }
    }
}

/// The blocks every path from which ends in a panic or in unwinding: a
/// cleanup block, one that ends in `unreachable` (after the call that
/// panicked, a contract's failure among them) or `resume`, and one whose
/// every successor is such; never the entry. Each is cold (decision 270).
fn cold_blocks(f: &sir::Function) -> Vec<bool> {
    let mut cold: Vec<bool> = f
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| i != 0 && (b.cleanup || matches!(b.term, Terminator::Unreachable | Terminator::Resume)))
        .collect();
    loop {
        let mut changed = false;
        for (i, b) in f.blocks.iter().enumerate() {
            if i == 0 || cold[i] {
                continue;
            }
            let all = match &b.term {
                Terminator::Goto(t) => cold.get(t.0 as usize).copied().unwrap_or(false),
                Terminator::Switch { cases, default, .. } => {
                    cases.iter().all(|(_, t)| cold.get(t.0 as usize).copied().unwrap_or(false))
                        && cold.get(default.0 as usize).copied().unwrap_or(false)
                }
                _ => false,
            };
            if all {
                cold[i] = true;
                changed = true;
            }
        }
        if !changed {
            return cold;
        }
    }
}

/// The texts an interpolation is building at the start of each block
/// (decision 381): the locals a `fmt.new()` made into on every path to the
/// block that nothing has moved, dropped, written over or ended since, and
/// that the block's cleanup does not drop. Lowering registers an
/// interpolation's `String` for its scope only once the statement that
/// builds it is done, so a panic or a `?` in one of its values leaves it
/// to no drop of the SIR: the interpreter's host frees it, and the native
/// code frees it on the failure path of each call, and before a `return`,
/// where such a text is still being built. Ascending by local.
fn building_texts(f: &sir::Function) -> Vec<Vec<u32>> {
    let n = f.blocks.len();
    let mut entry: Vec<Option<Vec<u32>>> = vec![None; n];
    if n == 0 {
        return Vec::new();
    }
    entry[0] = Some(Vec::new());
    let covered: Vec<Vec<u32>> = (0..n).map(|i| cleanup_drops(f, i)).collect();
    loop {
        let mut changed = false;
        for i in 0..n {
            if f.blocks[i].cleanup {
                continue;
            }
            let Some(start) = entry[i].clone() else { continue };
            let mut set: Vec<u32> = start.into_iter().filter(|l| !covered[i].contains(l)).collect();
            for s in &f.blocks[i].stmts {
                building_taken(&mut set, &s.kind);
                building_begun(&mut set, &s.kind, &covered[i]);
            }
            let succs: Vec<usize> = match &f.blocks[i].term {
                Terminator::Goto(t) => vec![t.0 as usize],
                Terminator::Switch { cases, default, .. } => {
                    let mut v: Vec<usize> = cases.iter().map(|(_, t)| t.0 as usize).collect();
                    v.push(default.0 as usize);
                    v
                }
                _ => Vec::new(),
            };
            for t in succs {
                if t >= n || f.blocks[t].cleanup {
                    continue;
                }
                let next = match &entry[t] {
                    None => set.clone(),
                    Some(old) => old.iter().copied().filter(|l| set.contains(l)).collect(),
                };
                if entry[t].as_ref() != Some(&next) {
                    entry[t] = Some(next);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    entry
        .into_iter()
        .enumerate()
        .map(|(i, e)| e.unwrap_or_default().into_iter().filter(|l| !covered[i].contains(l)).collect())
        .collect()
}

/// The locals the cleanup of block `i` drops, through its chain of cleanup
/// blocks to its `resume`.
fn cleanup_drops(f: &sir::Function, i: usize) -> Vec<u32> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    let mut next = f.blocks[i].unwind.map(|u| u.0 as usize);
    while let Some(b) = next {
        if b >= f.blocks.len() || seen.contains(&b) {
            break;
        }
        seen.push(b);
        for s in &f.blocks[b].stmts {
            if let StatementKind::Drop { place, .. } = &s.kind
                && !out.contains(&place.local.0)
            {
                out.push(place.local.0);
            }
        }
        next = match &f.blocks[b].term {
            Terminator::Goto(t) => Some(t.0 as usize),
            _ => None,
        };
    }
    out
}

/// What a statement takes of the texts being built, before it runs (and
/// before a failure inside it): a text it moves, drops, ends or writes over
/// is no longer the interpolation's to free (decision 381).
fn building_taken(set: &mut Vec<u32>, s: &StatementKind) {
    if set.is_empty() {
        return;
    }
    let mut ops: Vec<&Operand> = Vec::new();
    let mut gone: Vec<u32> = Vec::new();
    match s {
        StatementKind::Assign(p, r) => {
            gone.push(p.local.0);
            match r {
                Rvalue::Use(o) | Rvalue::Unary(_, o) => ops.push(o),
                Rvalue::Binary(_, a, b) => {
                    ops.push(a);
                    ops.push(b);
                }
                Rvalue::Aggregate(_, all) | Rvalue::Closure(_, _, all) => ops.extend(all),
                Rvalue::Ref(..) | Rvalue::Discriminant(_) | Rvalue::CopyOf(_) | Rvalue::StrView(_) => {}
            }
        }
        StatementKind::Call { dest, callee, args } => {
            gone.push(dest.local.0);
            if let Callee::Value(o) = callee {
                ops.push(o);
            }
            ops.extend(args);
        }
        StatementKind::Drop { place, .. } => gone.push(place.local.0),
        StatementKind::StorageDead(l) => gone.push(l.0),
    }
    for o in ops {
        if let Operand::Move(p) = o {
            gone.push(p.local.0);
        }
    }
    set.retain(|l| !gone.contains(l));
}

/// The text a statement begins to build: the local a `fmt.new()` makes
/// (decision 381), unless the block's cleanup drops it (`covered`).
fn building_begun(set: &mut Vec<u32>, s: &StatementKind, covered: &[u32]) {
    if let StatementKind::Call { dest, callee: Callee::Intrinsic(name, _), .. } = s
        && name == "fmt.new"
        && dest.proj.is_empty()
        && !set.contains(&dest.local.0)
        && !covered.contains(&dest.local.0)
    {
        set.push(dest.local.0);
        set.sort_unstable();
    }
}

fn drop_flag(l: &sir::LocalDecl) -> bool {
    l.name.as_deref() == Some("flag") && l.loc.is_none()
}

/// A sized integer type's operation an intrinsic names (`u8.wrap_add`,
/// or `int.to_u8` for the conversion to one): the type's number and the
/// operation.
fn sized_op(name: &str) -> Option<(u32, SizedOp)> {
    let (owner, method) = name.split_once('.')?;
    let kind = |k: &str| abi::SIZED_TYPES.iter().position(|t| *t == k).map(|i| i as u32);
    if owner == "int" {
        return Some((kind(method.strip_prefix("to_")?)?, SizedOp::ToSized));
    }
    Some((kind(owner)?, SizedOp::from_name(method)?))
}

/// An operation of `i128` or `u128` an intrinsic names (`u128.add`, or
/// `int.to_u128` for the conversion to one): the type's number in
/// [`abi::WIDE_TYPES`] and the operation.
fn wide_op(name: &str) -> Option<(u32, SizedOp)> {
    let (owner, method) = name.split_once('.')?;
    let kind = |k: &str| abi::WIDE_TYPES.iter().position(|t| *t == k).map(|i| i as u32);
    if owner == "int" {
        return Some((kind(method.strip_prefix("to_")?)?, SizedOp::ToSized));
    }
    Some((kind(owner)?, SizedOp::from_name(method)?))
}

/// What a reference (or a mutex guard) points to.
fn deref_type(t: &Type) -> Result<Type, Error> {
    match t {
        Type::Ref(inner, _) => Ok((**inner).clone()),
        Type::Named(n, a) if n == "MutexGuard" || n == "ReadGuard" || n == "Shared" => {
            Ok(a.first().cloned().unwrap_or(Type::Unit))
        }
        other => Err(Error::unsupported(format!("dereferencing a `{}`", sir::write::ty(other)))),
    }
}

/// What a value is behind its references.
fn target_type(t: &Type) -> Type {
    match t {
        Type::Ref(inner, _) => target_type(inner),
        other => other.clone(),
    }
}

/// The element type of a list or an array.
/// The bytes a value of `t` takes in raw memory and whether a byte of it is
/// signed, for the types whose raw memory the code reads and writes inline
/// (decision 335): a word for those whose raw memory is their value's word,
/// a byte for `u8` and `i8`.
fn raw_scalar(t: &Type) -> Option<(u32, bool)> {
    match t {
        Type::Int | Type::Float | Type::Bool | Type::Char => Some((8, false)),
        Type::Named(n, _) => match n.as_str() {
            "u64" | "ptr" | "ptr_mut" => Some((8, false)),
            "u8" => Some((1, false)),
            "i8" => Some((1, true)),
            _ => None,
        },
        _ => None,
    }
}

fn element_type(t: &Type) -> Result<Type, Error> {
    match t {
        Type::Named(n, a) if n == "list" || n == "Array" || n == "Slice" => {
            a.first().cloned().ok_or_else(|| Error::internal("a list without an element type"))
        }
        other => Err(Error::unsupported(format!("indexing a `{}`", sir::write::ty(other)))),
    }
}

/// Matches an impl's pattern (its parameters bound in `binds`) against a
/// concrete type (`sisu-interp/src/dispatch.rs`).
pub(crate) fn match_type(pat: &Type, ty: &Type, binds: &mut [Option<Type>]) -> bool {
    match pat {
        Type::Param(i) => match binds.get_mut(*i as usize) {
            Some(slot @ None) => {
                *slot = Some(ty.clone());
                true
            }
            Some(Some(b)) => b == ty,
            None => false,
        },
        Type::Named(n, a) => {
            matches!(ty, Type::Named(m, b) if m == n && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        Type::Tuple(a) => {
            matches!(ty, Type::Tuple(b) if a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        Type::Ref(a, m) => matches!(ty, Type::Ref(b, k) if m == k && match_type(a, b, binds)),
        Type::Fn(ps, r, pure) => matches!(ty, Type::Fn(qs, s, q) if pure == q && ps.len() == qs.len()
            && ps.iter().zip(qs).all(|(x, y)| match_type(x, y, binds)) && match_type(r, s, binds)),
        Type::Dyn(n, a) => {
            matches!(ty, Type::Dyn(m, b) if m == n && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| match_type(x, y, binds)))
        }
        other => other == ty,
    }
}

/// The literal a constant's function returns, when that is all it does:
/// one block that assigns it to the result and returns (`const LIMIT: int
/// = 5`).
fn returned_constant(f: &sir::Function) -> Option<&Const> {
    if f.kind != sir::FnKind::Value || f.param_count != 0 || !f.type_params.is_empty() {
        return None;
    }
    let [block] = f.blocks.as_slice() else { return None };
    let [stmt] = block.stmts.as_slice() else { return None };
    match (&stmt.kind, &block.term) {
        (StatementKind::Assign(p, Rvalue::Use(Operand::Const(c))), Terminator::Return)
            if p.local.0 == 0 && p.proj.is_empty() && !matches!(c, sir::Const::Fn(_)) =>
        {
            Some(c)
        }
        _ => None,
    }
}

/// The C type a foreign function takes or gives for a Sisu type
/// (22, "Foreign calls"): the integers and floats at their width, a raw
/// pointer and an option of one as an address; `None` for a type that has
/// no C form.
fn c_type(t: &Type) -> Option<ir::Type> {
    Some(match t {
        Type::Int => types::I64,
        Type::Float => types::F64,
        Type::Bool => types::I8,
        Type::Named(n, args) => match n.as_str() {
            "u64" => types::I64,
            "u8" | "i8" => types::I8,
            "u16" | "i16" => types::I16,
            "u32" | "i32" => types::I32,
            "f32" => types::F32,
            "ptr" | "ptr_mut" => types::I64,
            "Option" if matches!(args.first(), Some(Type::Named(p, _)) if p == "ptr" || p == "ptr_mut") => {
                types::I64
            }
            _ => return None,
        },
        _ => return None,
    })
}

/// The source-level name of a SIR function, for stack traces
/// (`sisu-interp/src/exec.rs`, `display_fn_name`).
fn display_fn_name(name: &str, prog: &sir::Program) -> String {
    sir::display_fn_name(name, prog)
}

/// How `==` compares two values of one variant of an enum compiled inline
/// ([`Cx::eq_shape`]).
enum Same {
    /// By their tags alone: the variant has no fields that hold anything.
    Tag,
    /// By the words at these offsets, which hold the fields the runtime
    /// compares as words.
    Words(Vec<u32>),
    /// By the runtime.
    Runtime,
}

/// What [`Cx::inline_intrinsic`] did with a call.
enum Fast {
    /// Nothing: the runtime runs it.
    No,
    /// All of it: the result is written.
    Done,
    /// The common case, ending at the block given; the builder is at the
    /// block where the runtime runs the rest.
    Slow(Block),
}
