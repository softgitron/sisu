//! One SIR function instance to Cranelift IR.
//!
//! Every local has a stack slot, except the result and the parameters,
//! whose memory the caller provides; a place is an address computed from
//! its local by its projections; an operand is the words at an address.
//! Each statement that can panic (a call, a drop, an overflow, an index)
//! is followed by a test of its status, which leads to the failure path of
//! its block: a line of the stack trace, then the block's cleanup, or the
//! return of [`abi::UNWINDING`]; in a cleanup block, the abort of a panic
//! during unwinding.

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
use std::collections::HashMap;

/// Values larger than this many words are copied through memory rather
/// than as words.
const MAX_WORDS: u32 = 16;

pub(crate) fn translate(
    cg: &mut Codegen<'_>,
    b: FunctionBuilder<'_>,
    f: &sir::Function,
    targs: &[Type],
) -> Result<(), Error> {
    let mut cx = Cx::new(cg, b, f, targs)?;
    cx.body()?;
    cx.finish()
}

enum Storage {
    /// The caller's memory, at this address.
    Param(Value),
    Slot(ir::StackSlot),
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
    /// The failure path of each SIR block, made when first needed.
    fails: Vec<Option<Block>>,
    line: Variable,
    col: Variable,
    funcs: HashMap<FuncId, FuncRef>,
    data: HashMap<DataId, GlobalValue>,
    /// The function's name and file for its stack trace lines; `None` for
    /// drop glue and the standard library, which traces leave out.
    trace: Option<(DataId, usize, DataId, usize)>,
    /// The function's file, for the site of a call that may wait.
    file: (DataId, usize),
    glue: bool,
    /// The SIR block being translated.
    cur: usize,
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

/// Whether a closure's environment of `size` bytes is passed as its one
/// word rather than its address (decision 213): an environment of one word
/// is copied with the function value, so it needs no cell, which nothing
/// could free (function values are `Copy` and never dropped). A task's
/// keeps its cell, which the runtime frees when the task ends.
fn inline_env(kind: sir::FnKind, size: u32) -> bool {
    size == 8 && matches!(kind, sir::FnKind::Closure(role) if role != sir::ClosureRole::Task)
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
        for (i, l) in f.locals.iter().enumerate() {
            let ty = cg.concrete(&l.ty, targs)?;
            let storage = if i == 1 && inline_env(f.kind, cg.types.size(&ty)?) {
                // The environment came as its one word (decision 213): it
                // gets a slot, as an argument has memory of its own.
                let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
                b.ins().stack_store(params[1], slot, 0);
                Storage::Slot(slot)
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
        let glue = f.kind == sir::FnKind::Glue;
        let file = cg.prog.sources.get(f.source as usize).cloned().unwrap_or_default();
        let std = file.starts_with("<std>/") || file.starts_with("<shims>/");
        let file_text = (cg.text(file.as_bytes())?, file.len());
        let trace = if glue || std {
            None
        } else {
            let name = display_fn_name(&f.name, cg.prog);
            let n = cg.text(name.as_bytes())?;
            Some((n, name.len(), file_text.0, file.len()))
        };
        let fails = vec![None; f.blocks.len()];
        let mut cx = Cx {
            cg,
            b,
            f,
            targs,
            locals,
            blocks,
            fails,
            line,
            col,
            funcs: HashMap::new(),
            data: HashMap::new(),
            trace,
            file: file_text,
            glue,
            cur: 0,
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
            None => Err(Error::internal(format!("no local _{}", l.0))),
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

    fn size(&mut self, t: &Type) -> Result<u32, Error> {
        self.cg.types.size(t)
    }

    fn type_id(&mut self, t: &Type) -> Result<u32, Error> {
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
        let counted = self.b.create_block();
        self.b.ins().brif(over, overflow, &[], counted, &[]);
        self.b.switch_to_block(overflow);
        self.store_word(d, a, 0);
        let st = self.rt_status(sym::PANIC_STACK, &[])?;
        self.b.ins().return_(&[st]);
        self.b.switch_to_block(counted);
        self.yield_check()?;
        let first = self.blocks[0];
        self.b.ins().jump(first, &[]);
        Ok(())
    }

    /// The stack probe (decision 203): the stack pointer, below the frame
    /// the prologue made, against the task's limit. Under it, a stack
    /// pointer at or above the real limit only crossed the decommit
    /// watermark: the task is marked dirty and the limit lowered. One below
    /// it has exhausted the stack: the function marks the failure, lowers
    /// the limit to the unwinding floor, and returns at once, calling
    /// nothing and writing nothing below its frame (the runtime raises the
    /// panic when it next looks at the panics in flight).
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
        let one = self.iconst(1);
        self.store_word(one, stack, abi::STACK_FAILED);
        let floor = self.load(stack, abi::STACK_FLOOR);
        self.store_word(floor, stack, abi::STACK_PROBE);
        let st = self.b.ins().iconst(types::I8, i64::from(abi::UNWINDING));
        self.b.ins().return_(&[st]);
        self.b.switch_to_block(cont);
    }

    /// A yield point (decision 85): one less of the budget, and the
    /// scheduler when it is spent.
    fn yield_check(&mut self) -> Result<(), Error> {
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
        self.rt(sym::YIELD, &[])?;
        self.b.ins().jump(cont, &[]);
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
        if let Some(b) = self.fails[self.cur] {
            return b;
        }
        let b = self.b.create_block();
        self.fails[self.cur] = Some(b);
        b
    }

    /// Continues when `status` is [`abi::OK`]; else takes the failure path.
    fn check(&mut self, status: Value) {
        let fail = self.fail_block();
        let cont = self.b.create_block();
        self.b.ins().brif(status, fail, &[], cont, &[]);
        self.b.switch_to_block(cont);
    }

    /// Continues when `ok` holds; else calls the runtime entry point that
    /// raises the panic (`raise`, with `args`) and takes the failure path.
    fn guard(&mut self, ok: Value, raise: &'static str, args: &[Value]) -> Result<(), Error> {
        let cont = self.b.create_block();
        let bad = self.b.create_block();
        self.b.ins().brif(ok, cont, &[], bad, &[]);
        self.b.switch_to_block(bad);
        self.rt(raise, args)?;
        let fail = self.fail_block();
        self.b.ins().jump(fail, &[]);
        self.b.switch_to_block(cont);
        Ok(())
    }

    fn emit_fails(&mut self) -> Result<(), Error> {
        for i in 0..self.fails.len() {
            let Some(fb) = self.fails[i] else { continue };
            self.b.switch_to_block(fb);
            let block = &self.f.blocks[i];
            if block.cleanup {
                self.rt(sym::ABORT_UNWIND, &[])?;
                self.b.ins().trap(ir::TrapCode::unwrap_user(1));
                continue;
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
            match block.unwind {
                Some(u) => {
                    let target = self.blocks[u.0 as usize];
                    self.b.ins().jump(target, &[]);
                }
                None => self.leave(abi::UNWINDING),
            }
        }
        Ok(())
    }

    // ---------------------------------------------------------------- body

    fn body(&mut self) -> Result<(), Error> {
        for (i, block) in self.f.blocks.iter().enumerate() {
            self.cur = i;
            let cb = self.blocks[i];
            self.b.switch_to_block(cb);
            for s in &block.stmts {
                self.cg.statements += 1;
                if let Some(loc) = s.loc {
                    self.set_loc(loc);
                }
                self.statement(&s.kind)?;
            }
            self.cg.statements += 1;
            if let Some(loc) = block.loc {
                self.set_loc(loc);
            }
            self.terminator(&block.term)?;
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
                    self.yield_check()?;
                }
                let target = self.blocks[bb.0 as usize];
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
                    self.yield_check()?;
                    let to = self.blocks[t];
                    self.b.ins().jump(to, &[]);
                }
            }
            Terminator::Return => self.leave(abi::OK),
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
            StatementKind::Assign(p, r) => {
                let dest = self.place_type(p)?;
                let size = self.size(&dest)?;
                let v = self.rvalue(r, &dest)?;
                let (addr, _) = self.dest_place(p)?;
                self.write(addr, &v, size);
                Ok(())
            }
            StatementKind::Call { dest, callee, args } => self.call(dest, callee, args),
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
        let mut addr = self.local_addr(p.local)?;
        let mut ty = self.locals[p.local.0 as usize].ty.clone();
        let mut variant: Option<u32> = None;
        for proj in &p.proj {
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
                    let len = self.load(addr, 8);
                    let at = if from_end { self.b.ins().isub(len, idx) } else { idx };
                    // In bounds: `0 <= i < len`, or `0 < i <= len` from the end.
                    let probe = if from_end { self.b.ins().iadd_imm(idx, -1) } else { idx };
                    let ok = self.b.ins().icmp(IntCC::UnsignedLessThan, probe, len);
                    let fe = self.iconst(i64::from(from_end));
                    self.guard(ok, sym::PANIC_INDEX, &[idx, len, fe])?;
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
                let env = self.iconst(0);
                (Val::Words(vec![code, env]), ty)
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

    /// A closure value: its function's code and the address of its
    /// environment, the tuple of its captures, or the environment itself
    /// when it is one word (decision 213). A closure that captures
    /// references cannot leave the frame of what it captures, so its
    /// environment is a slot of this frame; one that copies its captures
    /// (`own`) may, and its environment is on the heap, never freed
    /// (docs/port-debt.md, "Stage 2").
    fn closure(&mut self, name: &str, tys: &[Type], ops: &[Operand]) -> Result<Val, Error> {
        let targs: Vec<Type> = tys.iter().map(|t| self.concrete(t)).collect::<Result<_, _>>()?;
        let f = self
            .cg
            .prog
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
        // A task's closure (and a generator's) runs after the statement that
        // makes it, with its own environment: a heap cell, which the
        // runtime frees when the task ends.
        let task = f.kind == sir::FnKind::Closure(sir::ClosureRole::Task);
        let env = if size == 0 {
            self.iconst(0)
        } else if inline_env(f.kind, size) {
            // One word of captures is the environment itself (decision 213).
            let mut word = self.iconst(0);
            for (i, o) in ops.iter().enumerate() {
                let (_, ft) = self.field_of(&env_ty, None, i as u32)?;
                if self.size(&ft)? == 8 {
                    let (v, _) = self.operand(o)?;
                    word = self.first_word(&v)?;
                }
            }
            word
        } else {
            let all_refs =
                matches!(&env_ty, Type::Tuple(items) if items.iter().all(|t| matches!(t, Type::Ref(..))));
            let env = if all_refs && !task {
                self.slot(size)
            } else {
                let bytes = self.iconst(i64::from(size));
                self.rt_status(sym::ALLOC, &[bytes])?
            };
            for (i, o) in ops.iter().enumerate() {
                let (off, ft) = self.field_of(&env_ty, None, i as u32)?;
                let (v, vt) = self.operand(o)?;
                let fsize = self.size(&vt)?.min(self.size(&ft)?);
                let at = self.b.ins().iadd_imm(env, i64::from(off));
                self.write(at, &v, fsize);
            }
            env
        };
        Ok(Val::Words(vec![code, env]))
    }

    fn unary(&mut self, op: UnOp, a: &Operand) -> Result<Val, Error> {
        let (x, ty) = self.scalar(a)?;
        let k = scalar_kind(&ty);
        let r = match (op, k) {
            (UnOp::Neg, Some(Scalar::Int)) => {
                let min = self.b.ins().icmp_imm(IntCC::NotEqual, x, i64::MIN);
                let code = self.i32const(IntOp::Neg as u32);
                let zero = self.iconst(0);
                self.guard(min, sym::PANIC_OVERFLOW, &[code, x, zero])?;
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
            Scalar::Int => self.int_op(op, x, y)?,
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

    fn int_op(&mut self, op: BinOp, x: Value, y: Value) -> Result<Value, Error> {
        let checked = |op: BinOp| match op {
            BinOp::Add => Some(IntOp::Add),
            BinOp::Sub => Some(IntOp::Sub),
            BinOp::Mul => Some(IntOp::Mul),
            _ => None,
        };
        if let Some(iop) = checked(op) {
            let (r, of) = match iop {
                IntOp::Add => self.b.ins().sadd_overflow(x, y),
                IntOp::Sub => self.b.ins().ssub_overflow(x, y),
                _ => self.b.ins().smul_overflow(x, y),
            };
            let ok = self.b.ins().icmp_imm(IntCC::Equal, of, 0);
            let code = self.i32const(iop as u32);
            self.guard(ok, sym::PANIC_OVERFLOW, &[code, x, y])?;
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
            let out = self.slot(8);
            let code = self.i32const(iop as u32);
            let st = self.rt_status(sym::INT_OP, &[code, x, y, out])?;
            self.check(st);
            return Ok(self.load(out, 0));
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
                        let (v, _) = self.operand(o)?;
                        self.write(cell, &v, isize);
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
                }
            }
            Aggregate::List => {
                let elem = element_type(dest)?;
                let esize = self.size(&elem)?;
                let n = ops.len() as i64;
                let bytes = self.iconst(n * i64::from(esize));
                let data = self.rt_status(sym::ALLOC, &[bytes])?;
                for (i, o) in ops.iter().enumerate() {
                    let (v, _) = self.operand(o)?;
                    let at = self.b.ins().iadd_imm(data, i as i64 * i64::from(esize));
                    self.write(at, &v, esize);
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
        let ty = self.place_type(place)?;
        if !self.cg.types.needs_drop(&ty)? {
            return Ok(());
        }
        let done = self.b.create_block();
        if let Some(fl) = flag {
            let a = self.local_addr(fl)?;
            let set = self.load(a, 0);
            let go = self.b.create_block();
            self.b.ins().brif(set, go, &[], done, &[]);
            self.b.switch_to_block(go);
            let zero = self.iconst(0);
            self.store_word(zero, a, 0);
        }
        let (addr, ty) = self.place(place, Some(done))?;
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
            .prog
            .function(name)
            .ok_or_else(|| Error::internal(format!("the program has no function `{name}`")))?;
        let mut params = Vec::new();
        for i in 0..f.param_count as usize {
            params.push(self.cg.concrete(&f.locals[i + 1].ty, &targs)?);
        }
        let ret_ty = self.cg.concrete(f.ret(), &targs)?;
        let id = self.cg.instance(name, targs)?;
        let mut argv = self.arguments(args, derefs, adapt.then_some(params.as_slice()))?;
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
        self.check(st);
        for (addr, ty) in lent {
            if self.cg.types.needs_drop(&ty)? {
                let idv = self.type_id(&ty)?;
                let idv = self.i32const(idv);
                let st = self.rt_status(sym::DROP, &[addr, idv])?;
                self.check(st);
            }
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
        let Some(i) = Intrinsic::from_name(name) else {
            return Err(Error::unsupported(format!("the intrinsic `{name}`")));
        };
        self.cg.note_intrinsic(name, targs)?;
        let dest_ty = self.place_type(dest)?;
        let argv = self.arguments(args, derefs, None)?;
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
        Ok(())
    }

    /// A call of a function value: its code with its environment first.
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
        let env = self.load(addr, 8);
        let argv = self.arguments(args, 0, None)?;
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
        let out = self.slot(16);
        let (kv, ov, fv) = (self.i32const(kind), self.i32const(op as u32), self.i32const(flags));
        let st = self.rt_status(sym::SIZED, &[kv, ov, fv, words[0], words[1], out])?;
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
        Type::Named(n, a) if n == "MutexGuard" || n == "Shared" => {
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

/// The source-level name of a SIR function, for stack traces
/// (`sisu-interp/src/exec.rs`, `display_fn_name`).
fn display_fn_name(name: &str, prog: &sir::Program) -> String {
    if let Some(i) = name.strip_prefix("test#").and_then(|n| n.parse::<usize>().ok())
        && let Some((t, _)) = prog.tests.get(i)
    {
        return format!("test {}", sir::write::quote(t));
    }
    if let Some(owner) = name.strip_suffix(".__check_invariant") {
        return format!("the invariant of {owner}");
    }
    match name.split_once("#closure") {
        Some((owner, _)) => format!("closure in {}", display_fn_name(owner, prog)),
        None => name.to_string(),
    }
}
