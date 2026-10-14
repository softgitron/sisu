//! The run loop and statements.
//!
//! The loop is a trampoline (18, Phase 0: "an explicit per-task frame
//! stack, no host recursion for Sisu calls"): a call pushes a [`Frame`]
//! and the loop continues in it; a return pops it and stores the result in
//! the caller's destination. Before a frame's next statement, the loop
//! runs the drops the frame has pending (`drops.rs`), which may push the
//! frames of drop glue. A panic walks the frame stack: in each frame it
//! jumps to the current block's cleanup, which ends in `resume`, or, if the
//! block has none, leaves the frame once its pending drops are done.

use crate::value::Value;
use crate::{Frame, Interp, MAX_DEPTH, PanicInfo, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use sisu_sir::{Callee, Operand, Place, Rvalue, StatementKind, Terminator};
use std::rc::Rc;

pub(crate) enum Event<'p> {
    Continue,
    /// A yield point: a loop's back edge or a call (Q-34).
    Yield,
    /// The top frame returned (and was popped).
    Returned(Value, Option<&'p Place>),
    /// The top frame finished its cleanup (and was popped).
    Resumed(Box<PanicInfo>),
    /// The running task finished: the scheduler runs another.
    Switch,
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
        self.call_fn_tys(fi, args, Vec::new())
    }

    /// [`Interp::call_fn`] with type arguments.
    pub(crate) fn call_fn_tys(
        &mut self,
        fi: usize,
        args: Vec<Value>,
        tys: Vec<sir::Type>,
    ) -> Result<Value, Unwind> {
        let base = self.frames.len();
        self.push_frame(fi, args, None, tys)?;
        self.run(base)
    }

    pub(crate) fn call_by_name(&mut self, name: &str, args: Vec<Value>) -> Result<Value, Unwind> {
        let fi = self.fn_index(name)?;
        self.call_fn(fi, args)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn callee_of(
        &self,
        f: &Value,
        mut args: Vec<Value>,
    ) -> Result<(usize, Vec<Value>, Vec<sir::Type>), Unwind> {
        match self.target(f)? {
            Value::Fn(name) => Ok((self.fn_index(name)?, args, Vec::new())),
            Value::Closure(c) => {
                args.insert(0, Value::Tuple(c.env.clone()));
                Ok((self.fn_index(&c.name)?, args, c.tys.clone()))
            }
            other => Err(ice(format!("calling a value that is not a function: {other:?}"))),
        }
    }

    /// A type of the running function with its type arguments filled in,
    /// and its associated types (`<T as Trait>.Name`) replaced by what the
    /// impl for `T` binds them to.
    pub(crate) fn concrete(&self, t: &sir::Type) -> sir::Type {
        let t = match self.frames.last() {
            Some(f) if !f.tys.is_empty() => t.subst(&f.tys),
            _ => t.clone(),
        };
        if has_assoc(&t) { self.normalize_ty(&t) } else { t }
    }

    /// Replaces each associated type of a concrete type by its binding.
    pub(crate) fn normalize_ty(&self, t: &sir::Type) -> sir::Type {
        use sir::Type;
        match t {
            Type::Assoc(s, tr, args, name) => {
                let s = self.normalize_ty(s);
                let args: Vec<Type> = args.iter().map(|a| self.normalize_ty(a)).collect();
                for imp in &self.prog.impls {
                    if imp.trait_name != *tr {
                        continue;
                    }
                    let mut binds: Vec<Option<Type>> = vec![None; imp.params as usize];
                    if crate::dispatch::match_type(&imp.self_ty, &s, &mut binds)
                        && imp
                            .trait_args
                            .iter()
                            .zip(&args)
                            .all(|(p, a)| crate::dispatch::match_type(p, a, &mut binds))
                        && let Some((_, bound)) = imp.assoc.iter().find(|(n, _)| n == name)
                    {
                        let targs: Vec<Type> = binds.into_iter().map(|b| b.unwrap_or(Type::Unit)).collect();
                        return self.normalize_ty(&bound.subst(&targs));
                    }
                }
                Type::Assoc(Box::new(s), tr.clone(), args, name.clone())
            }
            Type::Named(n, args) => {
                Type::Named(n.clone(), args.iter().map(|a| self.normalize_ty(a)).collect())
            }
            Type::Dyn(n, args) => Type::Dyn(n.clone(), args.iter().map(|a| self.normalize_ty(a)).collect()),
            Type::Tuple(es) => Type::Tuple(es.iter().map(|e| self.normalize_ty(e)).collect()),
            Type::Ref(t, m) => Type::Ref(Box::new(self.normalize_ty(t)), *m),
            Type::Fn(ps, r, p) => Type::Fn(
                ps.iter().map(|x| self.normalize_ty(x)).collect(),
                Box::new(self.normalize_ty(r)),
                *p,
            ),
            other => other.clone(),
        }
    }

    pub(crate) fn push_frame(
        &mut self,
        fi: usize,
        args: Vec<Value>,
        ret: Option<&'p Place>,
        tys: Vec<sir::Type>,
    ) -> Result<(), Unwind> {
        let fr = self.make_frame(fi, args, ret, tys)?;
        self.push_built(fr)
    }

    /// A frame for a call of function `fi`, not pushed yet.
    pub(crate) fn make_frame(
        &mut self,
        fi: usize,
        args: Vec<Value>,
        ret: Option<&'p Place>,
        tys: Vec<sir::Type>,
    ) -> Result<Frame<'p>, Unwind> {
        let f = self.prog.functions.get(fi).ok_or_else(|| ice("no such function"))?;
        let mut locals = vec![Value::Moved; f.locals.len()];
        for (i, a) in args.into_iter().enumerate() {
            if let Some(slot) = locals.get_mut(i + 1) {
                *slot = a;
            }
        }
        self.serial += 1;
        Ok(Frame {
            serial: self.serial,
            func: Some(fi),
            locals,
            loc: None,
            bb: 0,
            next: 0,
            pending: None,
            ret,
            tys,
            drops: Vec::new(),
            after: None,
            resume_after_drops: false,
            anchor: None,
        })
    }

    pub(crate) fn push_built(&mut self, fr: Frame<'p>) -> Result<(), Unwind> {
        if self.frames.len() >= MAX_DEPTH {
            return Err(panic(msg::PANIC_STACK_OVERFLOW, &[("depth", MAX_DEPTH.to_string())]));
        }
        self.frames.push(fr);
        Ok(())
    }

    /// Runs until the frame at index `base` of the running task returns.
    /// Other tasks run in between, when this one waits or gives way.
    fn run(&mut self, base: usize) -> Result<Value, Unwind> {
        let root = self.sched.cur;
        loop {
            let cur = self.sched.cur as usize;
            let ev = match self.sched.tasks.get_mut(cur).and_then(|t| t.pending.take()) {
                // The call the task waited in, again.
                Some(pc) => self.retry(pc).map(|()| Event::Continue),
                None => self.step(),
            };
            self.sched.steps += 1;
            match ev {
                Ok(Event::Continue) => {
                    if self.sched.yield_asked && self.yield_point() {
                        self.preempt()?;
                    }
                }
                Ok(Event::Yield) => {
                    if self.yield_point() {
                        self.preempt()?;
                    }
                }
                Ok(Event::Returned(v, ret)) => {
                    if self.sched.cur == root && self.frames.len() <= base {
                        return Ok(v);
                    }
                    if let Some(dest) = ret
                        && let Err(e) = self.store(dest, v)
                    {
                        self.raise(e, base, root)?;
                    }
                }
                Ok(Event::Resumed(p)) => {
                    if self.sched.cur == root && self.frames.len() <= base {
                        return Err(Unwind::Panic(p));
                    }
                    self.raise(Unwind::Panic(p), base, root)?;
                }
                Ok(Event::Switch) | Err(Unwind::Switch) => self.switch()?,
                Err(e) => self.raise(e, base, root)?,
            }
        }
    }

    /// Retries the intrinsic call the running task waited in.
    fn retry(&mut self, pc: crate::sched::PendingCall<'p>) -> Result<(), Unwind> {
        let r = self.intrinsic(&pc.name, &pc.tys, pc.args, pc.ret.as_ref());
        self.finish_call(&pc.name, &pc.tys, pc.dest, pc.ret, r)
    }

    /// Stores what an intrinsic returned, or records the call when it must
    /// wait.
    fn finish_call(
        &mut self,
        name: &str,
        tys: &[sir::Type],
        dest: &'p Place,
        ret: Option<sir::Type>,
        r: Result<Value, Unwind>,
    ) -> Result<(), Unwind> {
        match r {
            Ok(v) => self.store(dest, v),
            Err(Unwind::Block(b)) => {
                let crate::sched::Blocked { args, wait, keys, timer } = *b;
                let cur = self.sched.cur as usize;
                if let Some(t) = self.sched.tasks.get_mut(cur) {
                    t.pending = Some(crate::sched::PendingCall {
                        name: name.to_string(),
                        tys: tys.to_vec(),
                        args,
                        dest,
                        ret,
                    });
                }
                Err(self.block_on(crate::sched::Blocked { args: Vec::new(), wait, keys, timer }))
            }
            Err(e) => Err(e),
        }
    }

    /// A panic in the top frame's current block: continue in its cleanup
    /// block, or leave the frame and repeat in the caller. Returns an error
    /// when the panic leaves the frame at `base` of task `root`, or aborts.
    /// The anchor of a task or a scope stops a panic: it records it, and
    /// its next step joins what it must before the panic goes on.
    fn raise(&mut self, e: Unwind, base: usize, root: u32) -> Result<(), Unwind> {
        let mut p = match e {
            Unwind::Panic(p) => p,
            Unwind::Switch | Unwind::Block(_) => return Err(ice("a task switch where a panic was expected")),
            abort => {
                if self.sched.cur == root {
                    self.frames.truncate(base);
                }
                return Err(abort);
            }
        };
        loop {
            let at_base = self.sched.cur == root && self.frames.len() <= base;
            if at_base {
                return Err(Unwind::Panic(p));
            }
            if let Some(anchor) = self.frames.last().and_then(|f| f.anchor) {
                match anchor {
                    crate::sched::Anchor::Shield => {
                        if let Some(fr) = self.frames.pop() {
                            let n = self.frames.len() as u32;
                            self.mon_frame_gone(fr.serial, n, fr.locals.len());
                        }
                        continue;
                    }
                    _ => {
                        if let Some(fr) = self.frames.last_mut() {
                            fr.pending = Some(p);
                        }
                        return Ok(());
                    }
                }
            }
            let (cleanup, unwind) = match self.current_block() {
                Some(b) => (b.cleanup, b.unwind),
                None => (false, None),
            };
            let draining = self.frames.last().is_some_and(|f| f.resume_after_drops);
            if cleanup || draining {
                let first = self
                    .frames
                    .last()
                    .and_then(|f| f.pending.as_ref())
                    .map(|q| q.message.clone())
                    .unwrap_or_default();
                if self.sched.cur == root {
                    self.frames.truncate(base);
                }
                return Err(Unwind::Abort(render(
                    msg::PANIC_DURING_UNWIND,
                    &[("first", first), ("message", p.message.clone())],
                )));
            }
            if !self.in_glue() {
                p.stack.push(self.frame_line());
            }
            match unwind {
                Some(u) => {
                    if let Some(fr) = self.frames.last_mut() {
                        fr.bb = u.0 as usize;
                        fr.next = 0;
                        fr.pending = Some(p);
                    }
                    return Ok(());
                }
                // Values the frame was dropping are still dropped.
                None if self.frames.last().is_some_and(|f| !f.drops.is_empty()) => {
                    if let Some(fr) = self.frames.last_mut() {
                        fr.resume_after_drops = true;
                        fr.pending = Some(p);
                    }
                    return Ok(());
                }
                None => {
                    if let Some(fr) = self.frames.pop() {
                        let n = self.frames.len() as u32;
                        self.mon_frame_gone(fr.serial, n, fr.locals.len());
                    }
                }
            }
        }
    }

    /// Whether the top frame runs drop glue that returns next, outside
    /// cleanup.
    fn glue_returns_next(&self) -> bool {
        let Some(fr) = self.frames.last() else { return false };
        let Some(f) = fr.func.and_then(|i| self.prog.functions.get(i)) else { return false };
        f.kind == sir::FnKind::Glue
            && fr.pending.is_none()
            && f.blocks.get(fr.bb).is_some_and(|b| fr.next >= b.stmts.len() && b.term == Terminator::Return)
    }

    /// Whether the top frame runs drop glue, which a stack trace leaves out.
    fn in_glue(&self) -> bool {
        let f = self.frames.last().and_then(|fr| fr.func).and_then(|i| self.prog.functions.get(i));
        f.is_some_and(|f| f.kind == sir::FnKind::Glue)
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
        let top = self.frames.len().checked_sub(1).ok_or_else(|| ice("a task without frames"))?;
        if let Some(d) = self.frames[top].drops.pop() {
            if top > 0 && self.frames[top].drops.is_empty() && self.glue_returns_next() {
                // The glue's last drop: the glue is done, and its caller
                // drops the part, so a chain of values takes one frame.
                if let Some(fr) = self.frames.pop() {
                    self.mon_frame_gone(fr.serial, top as u32, fr.locals.len());
                }
                self.frames[top - 1].drops.push(d);
                return Ok(Event::Continue);
            }
            self.drop_next(d)?;
            return Ok(Event::Continue);
        }
        if self.frames[top].resume_after_drops {
            let fr = self.frames.pop().ok_or_else(|| ice("resume without a frame"))?;
            let n = self.frames.len() as u32;
            self.mon_frame_gone(fr.serial, n, fr.locals.len());
            return fr.pending.map(Event::Resumed).ok_or_else(|| ice("resume outside unwinding"));
        }
        if let Some(a) = self.frames[top].anchor {
            return self.step_anchor(a);
        }
        let block = self.current_block().ok_or_else(|| ice("the top frame has no current block"))?;
        let next = self.frames[top].next;
        if let Some(s) = block.stmts.get(next) {
            if self.monitoring() {
                self.mon_cover(s.loc);
            }
            self.frames[top].next += 1;
            if s.loc.is_some() {
                self.frames[top].loc = s.loc;
            }
            self.tick()?;
            let depth = self.frames.len();
            self.stmt(s)?;
            // A call of a Sisu function is a yield point.
            return Ok(if self.frames.len() > depth { Event::Yield } else { Event::Continue });
        }
        if block.loc.is_some() {
            self.frames[top].loc = block.loc;
        }
        if self.monitoring() {
            self.mon_cover(block.loc);
        }
        let here = self.frames[top].bb;
        // A jump back is a loop's back edge: a yield point.
        let back = |t: usize| if t <= here { Event::Yield } else { Event::Continue };
        match &block.term {
            Terminator::Goto(t) => {
                self.jump(t.0 as usize);
                Ok(back(t.0 as usize))
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
                Ok(back(t.0 as usize))
            }
            Terminator::Return => {
                let mut fr = self.frames.pop().ok_or_else(|| ice("return without a frame"))?;
                let n = self.frames.len() as u32;
                self.mon_frame_gone(fr.serial, n, fr.locals.len());
                let v =
                    fr.locals.first_mut().map(|v| std::mem::replace(v, Value::Moved)).unwrap_or(Value::Unit);
                let v = if v.is_moved() { Value::Unit } else { v };
                if let Some(a) = fr.after.take() {
                    self.after(a, &v)?;
                }
                Ok(Event::Returned(v, fr.ret))
            }
            Terminator::Resume => {
                let fr = self.frames.pop().ok_or_else(|| ice("resume without a frame"))?;
                let n = self.frames.len() as u32;
                self.mon_frame_gone(fr.serial, n, fr.locals.len());
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
                Callee::Fn(name, tys) => {
                    let fi = self.fn_index(name)?;
                    let vals = self.operands(args)?;
                    // Passing a reference or a view is a use of its loan.
                    vals.iter().try_for_each(|v| self.mon_passed(v))?;
                    let tys = tys.iter().map(|t| self.concrete(t)).collect();
                    self.push_frame(fi, vals, Some(dest), tys)
                }
                Callee::Trait(name, tys) => {
                    let vals = self.operands(args)?;
                    vals.iter().try_for_each(|v| self.mon_passed(v))?;
                    let tys: Vec<sir::Type> = tys.iter().map(|t| self.concrete(t)).collect();
                    match self.dispatch(name, tys, vals)? {
                        crate::dispatch::Target::Fn(fi, vals, tys) => {
                            self.push_frame(fi, vals, Some(dest), tys)
                        }
                        crate::dispatch::Target::Intrinsic(n, vals, tys) => {
                            let ret = self.dest_type(dest);
                            let r = self.intrinsic(&n, &tys, vals, ret.as_ref());
                            self.finish_call(&n, &tys, dest, ret, r)
                        }
                    }
                }
                Callee::Value(op) => {
                    let f = self.operand(op)?;
                    let vals = self.operands(args)?;
                    vals.iter().try_for_each(|v| self.mon_passed(v))?;
                    self.mon_passed(&f)?;
                    let (fi, vals, tys) = self.callee_of(&f, vals)?;
                    self.push_frame(fi, vals, Some(dest), tys)
                }
                Callee::Intrinsic(name, tys) => {
                    let tys: Vec<sir::Type> = tys.iter().map(|t| self.concrete(t)).collect();
                    let tys = &tys;
                    let vals = self.operands(args)?;
                    self.mon_args(&vals)?;
                    // `list.clear` drops the elements: under the monitor, a
                    // must-consume one among them is the violation.
                    if name == "list.clear"
                        && self.monitoring()
                        && let Some(v) = vals.first()
                    {
                        let at = self.pointer_of(v)?;
                        self.mon_drop_unconsumed(&at, self.read(&at)?)?;
                    }
                    let ret = self.dest_type(dest);
                    // A scope's body runs as a frame above the scope's
                    // anchor, which gives the result to `dest`.
                    if matches!(name.as_str(), "task.scope" | "task.race" | "task.shield") {
                        return self.open_scope(name, vals, dest, ret);
                    }
                    if name == "test.catch" {
                        return self.open_catch(vals, dest);
                    }
                    let r = self.intrinsic(name, tys, vals, ret.as_ref());
                    self.finish_call(name, tys, dest, ret, r)
                }
            },
            StatementKind::Drop { place, flag } => {
                // A flagged drop runs when its flag is set, and clears it.
                if let Some(f) = flag {
                    match self.local(*f)? {
                        Value::Bool(true) => {}
                        Value::Bool(false) | Value::Moved => return Ok(()),
                        other => return Err(ice(format!("drop flag holds {other:?}"))),
                    }
                    if let Some(slot) = self.frames.last_mut().and_then(|fr| fr.locals.get_mut(f.0 as usize))
                    {
                        *slot = Value::Bool(false);
                    }
                }
                let Some(ptr) = self.resolve_for_drop(place)? else { return Ok(()) };
                self.mon_access(&ptr, crate::monitor::Op::Drop)?;
                let v = std::mem::replace(self.write(&ptr)?, Value::Moved);
                self.mon_drop_unconsumed(&ptr, &v)?;
                let ty = self.drop_type(place, &v);
                self.queue_drop(v, ty);
                Ok(())
            }
            StatementKind::StorageDead(l) => {
                if let Some(slot) = self.frames.last_mut().and_then(|f| f.locals.get_mut(l.0 as usize)) {
                    *slot = Value::Moved;
                }
                let n = self.frames.len().saturating_sub(1) as u32;
                if let Some(serial) = self.frames.last().map(|f| f.serial) {
                    self.mon_dead(serial, l.0, n);
                }
                Ok(())
            }
        }
    }

    /// Whether the statement being run stores into an unnamed local: a
    /// `mut` borrow into one is two-phase (as the checker has it).
    fn dest_is_temp(&self) -> bool {
        let prog = self.prog;
        let Some(fr) = self.frames.last() else { return false };
        let Some(f) = fr.func.and_then(|i| prog.functions.get(i)) else { return false };
        let Some(s) = f.blocks.get(fr.bb).and_then(|b| b.stmts.get(fr.next.wrapping_sub(1))) else {
            return false;
        };
        match &s.kind {
            StatementKind::Assign(p, _) => f.locals.get(p.local.0 as usize).is_some_and(|l| l.name.is_none()),
            _ => false,
        }
    }

    /// The declared type of a call's destination, when it is a whole local.
    /// The type of a call's destination, with the running function's type
    /// arguments filled in.
    fn dest_type(&self, dest: &Place) -> Option<sir::Type> {
        let prog = self.prog;
        if !dest.proj.is_empty() {
            return None;
        }
        let fi = self.frames.last()?.func?;
        prog.functions.get(fi)?.locals.get(dest.local.0 as usize).map(|l| self.concrete(&l.ty))
    }

    pub(crate) fn local(&self, l: sir::LocalId) -> Result<&Value, Unwind> {
        self.frames
            .last()
            .and_then(|f| f.locals.get(l.0 as usize))
            .ok_or_else(|| ice(format!("no local _{}", l.0)))
    }

    fn store(&mut self, p: &Place, v: Value) -> Result<(), Unwind> {
        let ptr = self.resolve_for_write(p)?;
        self.mon_access(&ptr, crate::monitor::Op::Write)?;
        *self.write(&ptr)? = v;
        Ok(())
    }

    /// The operands of one statement. Under the monitor, a copied `ref mut`
    /// reborrows only once every operand is read: a `mut` argument is
    /// two-phase, active at the call, as the checker has it.
    fn operands(&mut self, ops: &[Operand]) -> Result<Vec<Value>, Unwind> {
        let mut vals = Vec::with_capacity(ops.len());
        for o in ops {
            vals.push(self.operand_value(o)?);
        }
        for v in vals.iter_mut() {
            self.mon_copied(v)?;
        }
        Ok(vals)
    }

    pub(crate) fn operand(&mut self, o: &Operand) -> Result<Value, Unwind> {
        let mut v = self.operand_value(o)?;
        self.mon_copied(&mut v)?;
        Ok(v)
    }

    fn operand_value(&mut self, o: &Operand) -> Result<Value, Unwind> {
        match o {
            Operand::Const(c) => Ok(const_value(c)),
            Operand::Copy(p) => {
                let ptr = self.resolve(p)?;
                self.mon_access(&ptr, crate::monitor::Op::Read)?;
                let v = self.read(&ptr)?;
                if v.is_moved() {
                    return Err(self.moved_at(&ptr));
                }
                Ok(v.clone())
            }
            Operand::Move(p) => {
                let ptr = self.resolve(p)?;
                self.mon_access(&ptr, crate::monitor::Op::Move)?;
                let v = std::mem::replace(self.write(&ptr)?, Value::Moved);
                if v.is_moved() {
                    return Err(self.moved_at(&ptr));
                }
                Ok(v)
            }
        }
    }

    fn rvalue(&mut self, r: &'p Rvalue) -> Result<Value, Unwind> {
        match r {
            Rvalue::Use(o) => self.operand(o),
            Rvalue::Ref(p, m) => {
                let mut ptr = self.resolve(p)?;
                if self.monitoring() {
                    // Borrowing a moved-from place is a use after move.
                    if self.read(&ptr)?.is_moved() {
                        return Err(self.moved_at(&ptr));
                    }
                    ptr.tag = self.mon_borrow(&ptr, *m, !self.dest_is_temp())?;
                }
                let r = Value::Ref(Box::new(ptr));
                // Reborrowing a `ref dyn` made from a `ref T` keeps `T`.
                Ok(match self.fat_type_of_deref(p)? {
                    Some(ty) => Value::Dyn(Box::new(crate::value::DynVal { ty, value: r })),
                    None => r,
                })
            }
            Rvalue::Binary(op, a, b) => {
                let a = self.operand(a)?;
                let b = self.operand(b)?;
                // Comparing through references reads what they point to.
                self.mon_read_value(&a)?;
                self.mon_read_value(&b)?;
                self.binary(*op, &a, &b)
            }
            Rvalue::Unary(op, a) => {
                let a = self.operand(a)?;
                self.mon_read_value(&a)?;
                self.unary(*op, &a)
            }
            Rvalue::Aggregate(kind, ops) => {
                let vals = self.operands(ops)?;
                self.aggregate(kind, vals)
            }
            Rvalue::Discriminant(p) => {
                let ptr = self.resolve(p)?;
                self.mon_access(&ptr, crate::monitor::Op::Read)?;
                match self.target(self.read(&ptr)?)? {
                    Value::Variant(a) => Ok(Value::Int(a.variant as i64)),
                    other => Err(ice(format!("discriminant of {other:?}"))),
                }
            }
            Rvalue::Closure(name, tys, ops) => {
                let env = self.operands(ops)?;
                let tys = tys.iter().map(|t| self.concrete(t)).collect();
                Ok(Value::Closure(Rc::new(crate::value::ClosureVal {
                    name: Rc::from(name.as_str()),
                    tys,
                    env,
                })))
            }
            Rvalue::CopyOf(p) => {
                let ptr = self.resolve(p)?;
                self.mon_access(&ptr, crate::monitor::Op::Read)?;
                let v = self.target(self.read(&ptr)?)?;
                if v.is_moved() {
                    return Err(self.moved_at(&ptr));
                }
                let v = v.clone();
                let copy = self.copy_value(&v);
                // A copy of a `Shared` is an access to its count.
                for id in std::mem::take(&mut self.sched.copied_shared) {
                    self.note_shared_access(id)?;
                }
                Ok(copy)
            }
            Rvalue::StrView(p) => {
                let ptr = self.resolve(p)?;
                if self.target(self.read(&ptr)?)?.is_moved() {
                    return Err(self.moved_at(&ptr));
                }
                let tag = if self.monitoring() { self.mon_borrow(&ptr, false, false)? } else { 0 };
                let s = self.text(self.read(&ptr)?)?;
                Ok(Value::Str(std::rc::Rc::from(s), tag))
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
            sir::Aggregate::Dyn(t) => {
                let ty = self.concrete(t);
                let value = vals.into_iter().next().unwrap_or(Value::Unit);
                Value::Dyn(Box::new(crate::value::DynVal { ty, value }))
            }
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
                        self.queue_drop(old, None);
                    }
                }
                Value::Dict(Box::new(d))
            }
        })
    }

    // ---------------------------------------------------------------- copies

    /// A copy made by `.copy()`: a deep copy, except that a `Shared` gets
    /// another owner (and a `Weak` another handle) rather than a copy of
    /// its value (04, rule 5).
    pub(crate) fn copy_value(&mut self, v: &Value) -> Value {
        match v {
            Value::Shared(id) => {
                self.sched.copied_shared.push(*id);
                if let Some(c) = self.heap.get_mut(*id as usize) {
                    c.strong += 1;
                }
                Value::Shared(*id)
            }
            Value::Weak(id) => {
                if let Some(c) = self.heap.get_mut(*id as usize) {
                    c.weak += 1;
                }
                Value::Weak(*id)
            }
            Value::Struct(a) | Value::Variant(a) => {
                let fields = a.fields.iter().map(|f| self.copy_value(f)).collect();
                let b = Box::new(crate::value::Adt { fields, ..(**a).clone() });
                if matches!(v, Value::Struct(_)) { Value::Struct(b) } else { Value::Variant(b) }
            }
            Value::Tuple(xs) => Value::Tuple(xs.iter().map(|x| self.copy_value(x)).collect()),
            Value::List(xs) => Value::List(xs.iter().map(|x| self.copy_value(x)).collect()),
            Value::Dict(d) => {
                let mut d2 = (**d).clone();
                for e in d2.entries.iter_mut() {
                    e.1 = self.copy_value(&e.1);
                }
                Value::Dict(Box::new(d2))
            }
            Value::Cell(x) => Value::Cell(Box::new(self.copy_value(x))),
            Value::Mutex(m) => Value::Mutex(Box::new(crate::value::MutexVal {
                value: self.copy_value(&m.value),
                locked: false,
                holder: 0,
                poisoned: m.poisoned,
            })),
            Value::Sender(_) | Value::Receiver(_) => {
                self.copy_channel_end(v);
                v.clone()
            }
            Value::Arena(a) => {
                let slots = a.slots.iter().map(|(g, x)| (*g, self.copy_value(x))).collect();
                Value::Arena(Box::new(crate::value::ArenaVal { slots, free: a.free.clone() }))
            }
            Value::Dyn(d) => Value::Dyn(Box::new(crate::value::DynVal {
                ty: d.ty.clone(),
                value: self.copy_value(&d.value),
            })),
            other => other.clone(),
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

/// Whether a type names an associated type.
fn has_assoc(t: &sir::Type) -> bool {
    use sir::Type;
    match t {
        Type::Assoc(..) => true,
        Type::Named(_, a) | Type::Dyn(_, a) => a.iter().any(has_assoc),
        Type::Tuple(es) => es.iter().any(has_assoc),
        Type::Ref(t, _) => has_assoc(t),
        Type::Fn(ps, r, _) => ps.iter().any(has_assoc) || has_assoc(r),
        _ => false,
    }
}
