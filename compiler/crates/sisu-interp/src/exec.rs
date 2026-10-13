//! The run loop, statements and drop glue.
//!
//! The loop is a trampoline (18, Phase 0: "an explicit per-task frame
//! stack, no host recursion for Sisu calls"): a call pushes a [`Frame`]
//! and the loop continues in it; a return pops it and stores the result in
//! the caller's destination. A panic walks the frame stack: in each frame
//! it jumps to the current block's cleanup, which ends in `resume`, or, if
//! the block has none, leaves the frame at once.

use crate::value::{Pointer, Value};
use crate::{Frame, Interp, MAX_DEPTH, PanicInfo, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use sisu_sir::{Callee, Operand, Place, Rvalue, StatementKind, Terminator};
use std::rc::Rc;

enum Event<'p> {
    Continue,
    /// The top frame returned (and was popped).
    Returned(Value, Option<&'p Place>),
    /// The top frame finished its cleanup (and was popped).
    Resumed(Box<PanicInfo>),
}

impl<'p> Interp<'p, '_> {
    pub(crate) fn fn_index(&self, name: &str) -> Result<usize, Unwind> {
        self.fns.get(name).copied().ok_or_else(|| ice(format!("no function `{name}`")))
    }

    /// Calls a SIR function (for a closure, the environment tuple is the
    /// first argument) and runs it to completion. Used by `main`, tests
    /// and intrinsics that call back into Sisu; Sisu-to-Sisu calls push a
    /// frame on the running loop instead.
    pub(crate) fn call_fn(&mut self, fi: usize, args: Vec<Value>) -> Result<Value, Unwind> {
        let base = self.frames.len();
        self.push_frame(fi, args, None)?;
        self.run(base)
    }

    pub(crate) fn call_by_name(&mut self, name: &str, args: Vec<Value>) -> Result<Value, Unwind> {
        let fi = self.fn_index(name)?;
        self.call_fn(fi, args)
    }

    /// Calls a function value (a function constant or a closure) to completion.
    pub(crate) fn call_value(&mut self, f: &Value, args: Vec<Value>) -> Result<Value, Unwind> {
        let (fi, args) = self.callee_of(f, args)?;
        self.call_fn(fi, args)
    }

    fn callee_of(&self, f: &Value, mut args: Vec<Value>) -> Result<(usize, Vec<Value>), Unwind> {
        match self.target(f)? {
            Value::Fn(name) => Ok((self.fn_index(name)?, args)),
            Value::Closure(name, env) => {
                args.insert(0, Value::Tuple((**env).clone()));
                Ok((self.fn_index(name)?, args))
            }
            other => Err(ice(format!("calling a value that is not a function: {other:?}"))),
        }
    }

    fn push_frame(&mut self, fi: usize, args: Vec<Value>, ret: Option<&'p Place>) -> Result<(), Unwind> {
        if self.frames.len() >= MAX_DEPTH {
            return Err(panic(msg::PANIC_STACK_OVERFLOW, &[("depth", MAX_DEPTH.to_string())]));
        }
        let f = self.prog.functions.get(fi).ok_or_else(|| ice("no such function"))?;
        let mut locals = vec![Value::Moved; f.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            if let Some(slot) = locals.get_mut(i + 1) {
                *slot = a;
            }
        }
        self.serial += 1;
        self.frames.push(Frame {
            serial: self.serial,
            func: Some(fi),
            locals,
            loc: None,
            bb: 0,
            next: 0,
            pending: None,
            ret,
        });
        Ok(())
    }

    /// Runs `f` with `vals` in a scratch frame, passing pointers to them;
    /// returns the values as `f` left them, and its result.
    pub(crate) fn with_scratch<R>(
        &mut self,
        vals: Vec<Value>,
        f: impl FnOnce(&mut Self, Vec<Pointer>) -> Result<R, Unwind>,
    ) -> (Vec<Value>, Result<R, Unwind>) {
        self.serial += 1;
        let serial = self.serial;
        let frame = self.frames.len() as u32;
        let ptrs =
            (0..vals.len() as u32).map(|local| Pointer { frame, serial, local, path: Vec::new() }).collect();
        self.frames.push(Frame {
            serial,
            func: None,
            locals: vals,
            loc: None,
            bb: 0,
            next: 0,
            pending: None,
            ret: None,
        });
        let r = f(self, ptrs);
        self.frames.truncate(frame as usize + 1);
        let vals = self.frames.pop().map(|fr| fr.locals).unwrap_or_default();
        (vals, r)
    }

    /// Runs until the frame at index `base` returns.
    fn run(&mut self, base: usize) -> Result<Value, Unwind> {
        loop {
            match self.step() {
                Ok(Event::Continue) => {}
                Ok(Event::Returned(v, ret)) => {
                    if self.frames.len() <= base {
                        return Ok(v);
                    }
                    if let Some(dest) = ret
                        && let Err(e) = self.store(dest, v)
                    {
                        self.raise(e, base)?;
                    }
                }
                Ok(Event::Resumed(p)) => {
                    if self.frames.len() <= base {
                        return Err(Unwind::Panic(p));
                    }
                    self.raise(Unwind::Panic(p), base)?;
                }
                Err(e) => self.raise(e, base)?,
            }
        }
    }

    /// A panic in the top frame's current block: continue in its cleanup
    /// block, or leave the frame and repeat in the caller. Returns an error
    /// when the panic leaves the frame at `base`, or aborts.
    fn raise(&mut self, e: Unwind, base: usize) -> Result<(), Unwind> {
        let mut p = match e {
            Unwind::Panic(p) => p,
            abort => {
                self.frames.truncate(base);
                return Err(abort);
            }
        };
        loop {
            if self.frames.len() <= base {
                return Err(Unwind::Panic(p));
            }
            let (cleanup, unwind) = match self.current_block() {
                Some(b) => (b.cleanup, b.unwind),
                None => (false, None),
            };
            if cleanup {
                let first = self
                    .frames
                    .last()
                    .and_then(|f| f.pending.as_ref())
                    .map(|q| q.message.clone())
                    .unwrap_or_default();
                self.frames.truncate(base);
                return Err(Unwind::Abort(render(
                    msg::PANIC_DURING_UNWIND,
                    &[("first", first), ("message", p.message.clone())],
                )));
            }
            p.stack.push(self.frame_line());
            match unwind {
                Some(u) => {
                    if let Some(fr) = self.frames.last_mut() {
                        fr.bb = u.0 as usize;
                        fr.next = 0;
                        fr.pending = Some(p);
                    }
                    return Ok(());
                }
                None => {
                    self.frames.pop();
                }
            }
        }
    }

    fn current_block(&self) -> Option<&'p sir::BasicBlock> {
        let prog = self.prog;
        let fr = self.frames.last()?;
        prog.functions.get(fr.func?)?.blocks.get(fr.bb)
    }

    /// `name (file:line:col)` for the top frame, as a catalog line.
    fn frame_line(&self) -> String {
        let Some(fr) = self.frames.last() else { return String::new() };
        let Some(f) = fr.func.and_then(|i| self.prog.functions.get(i)) else { return String::new() };
        let file = self.prog.sources.get(f.source as usize).map(String::as_str).unwrap_or("?");
        let location = match fr.loc {
            Some(l) => format!("{file}:{}:{}", l.line, l.col),
            None => file.to_string(),
        };
        render(msg::RUN_FRAME, &[("function", display_fn_name(&f.name, self.prog)), ("location", location)])
    }

    fn tick(&mut self) -> Result<(), Unwind> {
        if let Some(f) = &mut self.fuel {
            if *f == 0 {
                return Err(Unwind::Abort(render(
                    msg::RUN_STEP_LIMIT,
                    &[("steps", self.fuel_limit.to_string())],
                )));
            }
            *f -= 1;
        }
        Ok(())
    }

    /// Executes one statement or terminator of the top frame.
    fn step(&mut self) -> Result<Event<'p>, Unwind> {
        let block = self.current_block().ok_or_else(|| ice("the top frame has no current block"))?;
        let top = self.frames.len() - 1;
        let next = self.frames[top].next;
        if let Some(s) = block.stmts.get(next) {
            self.frames[top].next += 1;
            if s.loc.is_some() {
                self.frames[top].loc = s.loc;
            }
            self.tick()?;
            self.stmt(s)?;
            return Ok(Event::Continue);
        }
        if block.loc.is_some() {
            self.frames[top].loc = block.loc;
        }
        match &block.term {
            Terminator::Goto(t) => {
                self.jump(t.0 as usize);
                Ok(Event::Continue)
            }
            Terminator::Switch { value, cases, default } => {
                let v = self.operand(value)?;
                let n = match self.target(&v)? {
                    Value::Int(n) => *n,
                    Value::Bool(b) => *b as i64,
                    other => return Err(ice(format!("switch on {other:?}"))),
                };
                let t = cases.iter().find(|(c, _)| *c == n).map(|(_, t)| *t).unwrap_or(*default);
                self.jump(t.0 as usize);
                Ok(Event::Continue)
            }
            Terminator::Return => {
                let mut fr = self.frames.pop().ok_or_else(|| ice("return without a frame"))?;
                let v =
                    fr.locals.first_mut().map(|v| std::mem::replace(v, Value::Moved)).unwrap_or(Value::Unit);
                Ok(Event::Returned(if v.is_moved() { Value::Unit } else { v }, fr.ret))
            }
            Terminator::Resume => {
                let fr = self.frames.pop().ok_or_else(|| ice("resume without a frame"))?;
                fr.pending.map(Event::Resumed).ok_or_else(|| ice("resume outside unwinding"))
            }
            Terminator::Unreachable => Err(panic(msg::PANIC_UNREACHABLE_PLAIN, &[])),
        }
    }

    fn jump(&mut self, bb: usize) {
        if let Some(fr) = self.frames.last_mut() {
            fr.bb = bb;
            fr.next = 0;
        }
    }

    fn stmt(&mut self, s: &'p sir::Statement) -> Result<(), Unwind> {
        match &s.kind {
            StatementKind::Assign(p, r) => {
                let v = self.rvalue(r)?;
                self.store(p, v)
            }
            StatementKind::Call { dest, callee, args } => match callee {
                Callee::Fn(name) => {
                    let fi = self.fn_index(name)?;
                    let vals = self.operands(args)?;
                    self.push_frame(fi, vals, Some(dest))
                }
                Callee::Value(op) => {
                    let f = self.operand(op)?;
                    let vals = self.operands(args)?;
                    let (fi, vals) = self.callee_of(&f, vals)?;
                    self.push_frame(fi, vals, Some(dest))
                }
                Callee::Intrinsic(name, tys) => {
                    let vals = self.operands(args)?;
                    let ret = self.dest_type(dest);
                    let v = self.intrinsic(name, tys, vals, ret)?;
                    self.store(dest, v)
                }
            },
            StatementKind::Drop { place, flag } => {
                if let Some(f) = flag {
                    match self.local(*f)? {
                        Value::Bool(true) => {}
                        Value::Bool(false) | Value::Moved => return Ok(()),
                        other => return Err(ice(format!("drop flag holds {other:?}"))),
                    }
                }
                let Some(ptr) = self.resolve_for_drop(place)? else { return Ok(()) };
                let v = std::mem::replace(self.write(&ptr)?, Value::Moved);
                self.drop_value(v)
            }
        }
    }

    /// The declared type of a call's destination, when it is a whole local.
    fn dest_type(&self, dest: &Place) -> Option<&'p sir::Type> {
        let prog = self.prog;
        if !dest.proj.is_empty() {
            return None;
        }
        let fi = self.frames.last()?.func?;
        prog.functions.get(fi)?.locals.get(dest.local.0 as usize).map(|l| &l.ty)
    }

    pub(crate) fn local(&self, l: sir::LocalId) -> Result<&Value, Unwind> {
        self.frames
            .last()
            .and_then(|f| f.locals.get(l.0 as usize))
            .ok_or_else(|| ice(format!("no local _{}", l.0)))
    }

    fn store(&mut self, p: &Place, v: Value) -> Result<(), Unwind> {
        let ptr = self.resolve_for_write(p)?;
        *self.write(&ptr)? = v;
        Ok(())
    }

    fn operands(&mut self, ops: &[Operand]) -> Result<Vec<Value>, Unwind> {
        ops.iter().map(|o| self.operand(o)).collect()
    }

    pub(crate) fn operand(&mut self, o: &Operand) -> Result<Value, Unwind> {
        match o {
            Operand::Const(c) => Ok(const_value(c)),
            Operand::Copy(p) => {
                let ptr = self.resolve(p)?;
                let v = self.read(&ptr)?;
                if v.is_moved() {
                    return Err(panic(msg::PANIC_MOVED_VALUE, &[]));
                }
                Ok(v.clone())
            }
            Operand::Move(p) => {
                let ptr = self.resolve(p)?;
                let v = std::mem::replace(self.write(&ptr)?, Value::Moved);
                if v.is_moved() {
                    return Err(panic(msg::PANIC_MOVED_VALUE, &[]));
                }
                Ok(v)
            }
        }
    }

    fn rvalue(&mut self, r: &'p Rvalue) -> Result<Value, Unwind> {
        match r {
            Rvalue::Use(o) => self.operand(o),
            Rvalue::Ref(p, _) => Ok(Value::Ref(Box::new(self.resolve(p)?))),
            Rvalue::Binary(op, a, b) => {
                let a = self.operand(a)?;
                let b = self.operand(b)?;
                self.binary(*op, &a, &b)
            }
            Rvalue::Unary(op, a) => {
                let a = self.operand(a)?;
                self.unary(*op, &a)
            }
            Rvalue::Aggregate(kind, ops) => {
                let vals = self.operands(ops)?;
                self.aggregate(kind, vals)
            }
            Rvalue::Discriminant(p) => {
                let ptr = self.resolve(p)?;
                match self.target(self.read(&ptr)?)? {
                    Value::Variant(a) => Ok(Value::Int(a.variant as i64)),
                    other => Err(ice(format!("discriminant of {other:?}"))),
                }
            }
            Rvalue::Closure(name, ops) => {
                let env = self.operands(ops)?;
                Ok(Value::Closure(Rc::from(name.as_str()), Rc::new(env)))
            }
            Rvalue::CopyOf(p) => {
                let ptr = self.resolve(p)?;
                let v = self.target(self.read(&ptr)?)?;
                if v.is_moved() {
                    return Err(panic(msg::PANIC_MOVED_VALUE, &[]));
                }
                Ok(v.clone())
            }
            Rvalue::StrView(p) => {
                let ptr = self.resolve(p)?;
                let s = self.text(self.read(&ptr)?)?;
                Ok(Value::str(s))
            }
        }
    }

    fn aggregate(&mut self, kind: &sir::Aggregate, vals: Vec<Value>) -> Result<Value, Unwind> {
        Ok(match kind {
            sir::Aggregate::Tuple => Value::Tuple(vals),
            sir::Aggregate::Struct(name) => Value::Struct(Value::adt(name, 0, vals)),
            sir::Aggregate::Variant(t, v) => {
                let i = self
                    .variants
                    .get(&(t.as_str(), v.as_str()))
                    .copied()
                    .ok_or_else(|| ice(format!("no variant {t}::{v}")))?;
                Value::variant(t, i, vals)
            }
            sir::Aggregate::List => Value::List(vals),
            sir::Aggregate::Set => {
                let mut s = crate::value::Set::default();
                for v in vals {
                    let k = self.key_of(&v)?;
                    s.insert(k, v);
                }
                Value::Set(Box::new(s))
            }
            sir::Aggregate::Dict => {
                let mut d = crate::value::Dict::default();
                let mut it = vals.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    let key = self.key_of(&k)?;
                    if let Some(old) = d.insert(key, k, v) {
                        self.drop_value(old)?;
                    }
                }
                Value::Dict(Box::new(d))
            }
        })
    }

    // ---------------------------------------------------------------- drop glue

    /// Runs the drop glue of a value: the type's `Drop` with a reference
    /// to the value, then its parts: fields and tuple elements in reverse
    /// order (04, rule 6), collection elements first to last, a dict
    /// entry's value before its key.
    pub(crate) fn drop_value(&mut self, v: Value) -> Result<(), Unwind> {
        if !self.any_drop {
            return Ok(());
        }
        match v {
            Value::Struct(a) | Value::Variant(a) if self.drop_fn(&a.ty).is_some() => {
                let is_struct = matches!(self.types.get(&*a.ty), Some(sir::TypeDecl::Struct { .. }));
                let name = self.drop_fn(&a.ty).unwrap_or_default();
                let v = if is_struct { Value::Struct(a) } else { Value::Variant(a) };
                let (vals, r) = self.with_scratch(vec![v], |me, ptrs| {
                    let arg = ptrs.into_iter().next().map(|p| Value::Ref(Box::new(p))).unwrap_or(Value::Unit);
                    me.call_by_name(&name, vec![arg])
                });
                r?;
                for v in vals {
                    if let Value::Struct(a) | Value::Variant(a) = v {
                        self.drop_parts(a.fields.into_iter().rev())?;
                    }
                }
                Ok(())
            }
            Value::Struct(a) | Value::Variant(a) => self.drop_parts(a.fields.into_iter().rev()),
            Value::Tuple(xs) => self.drop_parts(xs.into_iter().rev()),
            Value::List(xs) => self.drop_parts(xs.into_iter()),
            Value::Set(s) => self.drop_parts(s.items.into_iter()),
            Value::Dict(d) => {
                for (k, v) in d.entries {
                    self.drop_value(v)?;
                    self.drop_value(k)?;
                }
                Ok(())
            }
            Value::Iter(it) => {
                let parts = crate::iter::owned_parts(*it);
                self.drop_parts(parts.into_iter())
            }
            Value::Entry(e) => self.drop_value(e.key),
            _ => Ok(()),
        }
    }

    fn drop_parts(&mut self, parts: impl Iterator<Item = Value>) -> Result<(), Unwind> {
        for p in parts {
            self.drop_value(p)?;
        }
        Ok(())
    }

    fn drop_fn(&self, ty: &str) -> Option<String> {
        match self.types.get(ty)? {
            sir::TypeDecl::Struct { drop, .. } | sir::TypeDecl::Enum { drop, .. } => drop.clone(),
        }
    }
}

/// The source-level name of a SIR function, for stack traces.
fn display_fn_name(name: &str, prog: &sir::Program) -> String {
    if let Some(i) = name.strip_prefix("test#").and_then(|n| n.parse::<usize>().ok())
        && let Some((t, _)) = prog.tests.get(i)
    {
        return format!("test {}", sisu_sir::write::quote(t));
    }
    match name.split_once("#closure") {
        Some((owner, _)) => format!("closure in {}", display_fn_name(owner, prog)),
        None => name.to_string(),
    }
}

pub(crate) fn const_value(c: &sir::Const) -> Value {
    match c {
        sir::Const::Int(n) => Value::Int(*n),
        sir::Const::Float(f) => Value::Float(*f),
        sir::Const::Bool(b) => Value::Bool(*b),
        sir::Const::Char(c) => Value::Char(*c),
        sir::Const::Str(s) => Value::str(s),
        sir::Const::Unit => Value::Unit,
        sir::Const::Fn(n) => Value::Fn(Rc::from(n.as_str())),
    }
}
