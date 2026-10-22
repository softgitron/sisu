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

use crate::value::{IterState, Value};
use crate::{Frame, Interp, MAX_DEPTH, PanicInfo, Unwind, ice, msg, panic, render};
use sisu_sir as sir;
use sisu_sir::{Callee, Operand, Place, Proj, Rvalue, StatementKind, Terminator};

/// How many emptied locals vectors the interpreter keeps for reuse.
const SPARE_LOCALS: usize = 64;
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
    /// The body of the function named `name` in the epoch the running
    /// task has adopted (`live.rs`).
    pub(crate) fn fn_index(&self, name: &str) -> Result<usize, Unwind> {
        self.fn_index_in(self.task_epoch(), name)
    }

    /// [`Interp::fn_index`] for a callee named in the program's SIR.
    fn callee_index(&mut self, name: &'p str) -> Result<usize, Unwind> {
        let key = (self.task_epoch(), name.as_ptr() as usize, name.len());
        if let Some(i) = self.callees.get(&key) {
            return Ok(*i);
        }
        let i = self.fn_index_in(key.0, name)?;
        self.callees.insert(key, i);
        Ok(i)
    }

    pub(crate) fn fn_index_in(&self, epoch: usize, name: &str) -> Result<usize, Unwind> {
        self.epochs
            .get(epoch)
            .and_then(|t| t.get(name))
            .copied()
            .ok_or_else(|| ice(format!("no function `{name}`")))
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
        let mut locals = self.spare_locals.pop().unwrap_or_default();
        locals.push(Value::Moved);
        locals.extend(args);
        self.frame_with(fi, locals, ret, tys)
    }

    /// A frame for a call of function `fi` whose locals start with the
    /// result's slot and the arguments: the rest are added, empty (and
    /// arguments past the function's locals dropped).
    fn frame_with(
        &mut self,
        fi: usize,
        mut locals: Vec<Value>,
        ret: Option<&'p Place>,
        tys: Vec<sir::Type>,
    ) -> Result<Frame<'p>, Unwind> {
        let f = self.func(fi).ok_or_else(|| ice("no such function"))?;
        let n = f.locals.len();
        locals.truncate(n);
        locals.resize_with(n, || Value::Moved);
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
            stack_bytes: 0,
        })
    }

    pub(crate) fn push_built(&mut self, mut fr: Frame<'p>) -> Result<(), Unwind> {
        if self.frames.len() >= MAX_DEPTH {
            return Err(panic(msg::PANIC_STACK_OVERFLOW, &[("depth", MAX_DEPTH.to_string())]));
        }
        if self.stats_kind == crate::StatsKind::StackDepth && self.sched.keep_stats {
            fr.stack_bytes =
                self.frames.last().map(|f| f.stack_bytes).unwrap_or(0) + 16 * fr.locals.len() as u64 + 64;
            let depth = self.frames.len() + 1;
            let s = &mut self.sched.stats;
            let bucket = (usize::BITS - depth.leading_zeros()) as usize - 1;
            if s.depths.len() <= bucket {
                s.depths.resize(bucket + 1, 0);
            }
            s.depths[bucket] += 1;
            if depth > s.max_depth {
                s.max_depth = depth;
                s.max_stack_bytes = fr.stack_bytes;
            }
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
            // A trace names the program's frames: the standard library's
            // (`check`'s loop around a property) are left out.
            if !self.in_glue() && !self.in_std() {
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
        let Some(f) = fr.func.and_then(|i| self.func(i)) else { return false };
        f.kind == sir::FnKind::Glue
            && fr.pending.is_none()
            && f.blocks.get(fr.bb).is_some_and(|b| fr.next >= b.stmts.len() && b.term == Terminator::Return)
    }

    /// Whether the top frame runs drop glue, which a stack trace leaves out.
    fn in_glue(&self) -> bool {
        let f = self.frames.last().and_then(|fr| fr.func).and_then(|i| self.func(i));
        f.is_some_and(|f| f.kind == sir::FnKind::Glue)
    }

    fn current_block(&self) -> Option<&'p sir::BasicBlock> {
        let fr = self.frames.last()?;
        self.func(fr.func?)?.blocks.get(fr.bb)
    }

    /// Whether the top frame runs a function of the standard library.
    fn in_std(&self) -> bool {
        let Some(fr) = self.frames.last() else { return false };
        let Some(f) = fr.func.and_then(|i| self.func(i)) else { return false };
        // The interceptors and fakes the compiler writes for the program's
        // traits are left out with the standard library.
        self.prog
            .sources
            .get(f.source as usize)
            .is_some_and(|s| s.starts_with("<std>/") || s.starts_with("<shims>/"))
    }

    /// `name (file:line:col)` for the top frame, as a catalog line.
    fn frame_line(&self) -> String {
        let Some(fr) = self.frames.last() else { return String::new() };
        let Some(f) = fr.func.and_then(|i| self.func(i)) else { return String::new() };
        let file = self.prog.sources.get(f.source as usize).map(String::as_str).unwrap_or("?");
        let location = match fr.loc {
            Some(l) => format!("{file}:{}:{}", l.line, l.col),
            None => file.to_string(),
        };
        render(msg::RUN_FRAME, &[("function", display_fn_name(&f.name, self.prog)), ("location", location)])
    }

    fn tick(&mut self) -> Result<(), Unwind> {
        // A property's case past its budget (`test.catch`) panics; the
        // budget is spent, so the unwinding's drops run without one.
        if let Some((end, steps)) = self.case_ends.last_mut()
            && self.sched.steps >= *end
        {
            *end = u64::MAX;
            return Err(Unwind::Panic(Box::new(crate::PanicInfo {
                message: render(msg::RUN_CASE_TOO_LONG, &[("steps", steps.to_string())]),
                stack: Vec::new(),
                hole: None,
                refinement: None,
                too_long: Some(*steps),
            })));
        }
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
                let n = match self.scalar(value) {
                    Some(Value::Int(n)) => n,
                    Some(Value::Bool(b)) => b as i64,
                    _ => {
                        let v = self.operand(value)?;
                        match self.target(&v)? {
                            Value::Int(n) => *n,
                            Value::Bool(b) => *b as i64,
                            other => return Err(ice(format!("switch on {other:?}"))),
                        }
                    }
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
                // The locals' storage goes to the next frame.
                let mut locals = std::mem::take(&mut fr.locals);
                locals.clear();
                if self.spare_locals.len() < SPARE_LOCALS {
                    self.spare_locals.push(locals);
                }
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
                if let Some(v) = self.scalar_rvalue(r) {
                    return self.store(p, v);
                }
                let v = self.rvalue(r)?;
                self.store(p, v)
            }
            StatementKind::Call { dest, callee, args } => match callee {
                Callee::Fn(name, tys) => {
                    let fi = self.callee_index(name)?;
                    if self.monitor.is_none() {
                        // The arguments go straight to the new frame's
                        // locals (the monitor's hooks are all that the
                        // general path adds).
                        let mut locals = self.spare_locals.pop().unwrap_or_default();
                        locals.push(Value::Moved);
                        for o in args {
                            match self.operand_value(o) {
                                Ok(v) => locals.push(v),
                                Err(e) => {
                                    locals.clear();
                                    self.spare_locals.push(locals);
                                    return Err(e);
                                }
                            }
                        }
                        let tys = tys.iter().map(|t| self.concrete(t)).collect();
                        let fr = self.frame_with(fi, locals, Some(dest), tys)?;
                        return self.push_built(fr);
                    }
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
                    // The `sim` profile may fail the call with an error of
                    // its declared set instead (17).
                    if let Some(e) = self.sim_fault(name, &tys, &vals, dest) {
                        return self.store(dest, Value::err(e));
                    }
                    match self.dispatch(name, tys, vals)? {
                        crate::dispatch::Target::Fn(fi, vals, tys) => {
                            self.push_frame(fi, vals, Some(dest), tys)
                        }
                        crate::dispatch::Target::Intrinsic(n, vals, tys) => {
                            let ret = if crate::intrinsics::needs_return_type(&n) {
                                self.dest_type(dest)
                            } else {
                                None
                            };
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
                    if let Some(r) = self.quick_intrinsic(name, args) {
                        return self.store(dest, r?);
                    }
                    // Type arguments without parameters are concrete as
                    // they are.
                    let owned: Vec<sir::Type>;
                    let tys: &[sir::Type] = if tys.iter().any(sir::Type::has_params) {
                        owned = tys.iter().map(|t| self.concrete(t)).collect();
                        &owned
                    } else {
                        tys
                    };
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
                    let ret =
                        if crate::intrinsics::needs_return_type(name) { self.dest_type(dest) } else { None };
                    // A scope's body runs as a frame above the scope's
                    // anchor, which gives the result to `dest`.
                    if matches!(name.as_str(), "task.scope" | "task.race" | "task.shield") {
                        return self.open_scope(name, vals, dest, ret);
                    }
                    if name == "test.catch" {
                        return self.open_catch(vals, dest);
                    }
                    if name == "sim.always" {
                        return self.open_always(vals, dest);
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
        let Some(fr) = self.frames.last() else { return false };
        let Some(f) = fr.func.and_then(|i| self.func(i)) else { return false };
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
    /// A fault of the `sim` profile for a capability call (17, "Faults come
    /// from types"): at the run's fault rate, a call through the operations
    /// trait of a handle on one of the simulation's own objects (the file
    /// system and network of `System.sim` and of a `Sim`'s nodes) gives an
    /// error of the method's declared error set, never another; `TaskError`
    /// is the scheduler's, and is left out. The fakes a test makes itself
    /// are left alone. A field of the error is the call's path if it takes
    /// one, else a value that says the simulation made it.
    fn sim_fault(&mut self, name: &str, tys: &[sir::Type], args: &[Value], dest: &Place) -> Option<Value> {
        let rate = self.sim.filter(|r| *r > 0)?;
        let (tr, _) = name.rsplit_once('.')?;
        if !matches!(tr, "FsOps" | "NetOps") {
            return None;
        }
        let handle = match tys.first()? {
            sir::Type::Named(n, a) if a.is_empty() && matches!(n.as_str(), "Fs" | "Net") => args.first()?,
            _ => return None,
        };
        let object = match self.target(handle).ok()? {
            Value::Cap(id) => match &self.heap.get(*id as usize)?.value {
                Value::Dyn(d) => match &d.ty {
                    sir::Type::Named(n, _) => n.clone(),
                    _ => return None,
                },
                _ => return None,
            },
            _ => return None,
        };
        if !matches!(object.as_str(), "cap.SimFs" | "cap.SimNetwork" | "sim.SimNet") {
            return None;
        }
        let errs: Vec<String> = match self.dest_type(dest)? {
            sir::Type::Named(n, a) if n == "Result" => match a.get(1)? {
                sir::Type::ErrSet(names) => names.clone(),
                sir::Type::Named(e, _) => vec![e.clone()],
                _ => return None,
            },
            _ => return None,
        };
        let errs: Vec<String> = errs.into_iter().filter(|e| e != "TaskError").collect();
        if errs.is_empty() || self.fault_rng.next() % 1_000_000 >= u64::from(rate) {
            return None;
        }
        let decl = &errs[self.fault_rng.below(errs.len())];
        let prog = self.prog;
        let Some(sir::TypeDecl::Enum { variants, .. }) = prog.types.iter().find(|t| t.name() == decl) else {
            return None;
        };
        if variants.is_empty() {
            return None;
        }
        let k = self.fault_rng.below(variants.len());
        let path = args.iter().find_map(|a| match self.target(a).ok()? {
            Value::Path(p) => Some(p.clone()),
            _ => None,
        });
        let fields = variants[k]
            .fields
            .iter()
            .map(|(_, t)| match t {
                sir::Type::Named(n, _) if n == "Path" => {
                    Value::Path(path.clone().unwrap_or_else(|| std::rc::Rc::from("(simulated)")))
                }
                sir::Type::Named(n, _) if n == "String" => {
                    Value::String("a fault the simulation made".into())
                }
                sir::Type::Int => Value::Int(0),
                sir::Type::Bool => Value::Bool(false),
                _ => Value::Unit,
            })
            .collect();
        Some(Value::variant(decl, k as u32, fields))
    }

    fn dest_type(&self, dest: &Place) -> Option<sir::Type> {
        if !dest.proj.is_empty() {
            return None;
        }
        let fi = self.frames.last()?.func?;
        self.func(fi)?.locals.get(dest.local.0 as usize).map(|l| self.concrete(&l.ty))
    }

    pub(crate) fn local(&self, l: sir::LocalId) -> Result<&Value, Unwind> {
        self.frames
            .last()
            .and_then(|f| f.locals.get(l.0 as usize))
            .ok_or_else(|| ice(format!("no local _{}", l.0)))
    }

    fn store(&mut self, p: &Place, v: Value) -> Result<(), Unwind> {
        // A whole local of the top frame, outside the monitor: in place.
        if p.proj.is_empty()
            && self.monitor.is_none()
            && let Some(slot) = self.frames.last_mut().and_then(|f| f.locals.get_mut(p.local.0 as usize))
        {
            *slot = v;
            return Ok(());
        }
        let ptr = self.resolve_for_write(p)?;
        self.mon_access(&ptr, crate::monitor::Op::Write)?;
        *self.write(&ptr)? = v;
        Ok(())
    }

    /// The operands of one statement. Under the monitor, a copied `ref mut`
    /// reborrows only once every operand is read: a `mut` argument is
    /// two-phase, active at the call, as the checker has it.
    fn operands(&mut self, ops: &'p [Operand]) -> Result<Vec<Value>, Unwind> {
        let mut vals = Vec::with_capacity(ops.len());
        for o in ops {
            vals.push(self.operand_value(o)?);
        }
        for v in vals.iter_mut() {
            self.mon_copied(v)?;
        }
        Ok(vals)
    }

    pub(crate) fn operand(&mut self, o: &'p Operand) -> Result<Value, Unwind> {
        let mut v = self.operand_value(o)?;
        self.mon_copied(&mut v)?;
        Ok(v)
    }

    /// The value an operand copies, read in place outside the monitor;
    /// `None` for a constant, a move, and a read that fails.
    fn peek(&self, o: &Operand) -> Option<&Value> {
        match o {
            Operand::Copy(p) => self.read_fast(p)?.ok().filter(|v| !v.is_moved()),
            Operand::Const(_) | Operand::Move(_) => None,
        }
    }

    /// The text an operand copies (a `str`, `String` or `Path`, through
    /// references), read in place outside the monitor.
    fn peek_text(&self, o: &Operand) -> Option<&str> {
        match self.target(self.peek(o)?).ok()? {
            Value::Str(t, _) | Value::Path(t) => Some(t),
            Value::String(t) => Some(t),
            _ => None,
        }
    }

    /// Whether an operand can be evaluated without failing: a constant, or
    /// a place that holds a value.
    fn evaluates(&self, o: &Operand) -> bool {
        match o {
            Operand::Const(_) => true,
            Operand::Copy(p) | Operand::Move(p) => {
                self.read_fast(p).is_some_and(|v| v.is_ok_and(|v| !v.is_moved()))
            }
        }
    }

    /// The intrinsics the bootstrap calls most, called directly outside
    /// the monitor: each gives what the general path ([`Interp::intrinsic`])
    /// gives and has its effects, without its dispatch on the name and
    /// without collecting the arguments. `None`, before any effect, for
    /// any other call and for one that would fail, which the general path
    /// then makes and reports.
    fn quick_intrinsic(&mut self, name: &str, args: &'p [Operand]) -> Option<Result<Value, Unwind>> {
        if self.monitor.is_some() {
            return None;
        }
        match (name, args) {
            ("str.byte_at", [s, i]) => {
                let Value::Int(i) = self.target(self.peek(i)?).ok()? else { return None };
                let text = match self.target(self.peek(s)?).ok()? {
                    Value::Str(t, _) => &**t,
                    Value::String(t) => t.as_str(),
                    Value::Path(t) => &**t,
                    _ => return None,
                };
                let b = usize::try_from(*i).ok().and_then(|i| text.as_bytes().get(i))?;
                Some(Ok(Value::Int(i64::from(*b))))
            }
            // A `u8` is an `int` in its range (`sized.rs`).
            ("u8.to_int", [x]) => match self.target(self.peek(x)?).ok()? {
                Value::Int(n) => Some(Ok(Value::Int(*n))),
                _ => None,
            },
            ("char.code", [c]) => match self.target(self.peek(c)?).ok()? {
                Value::Char(c) => Some(Ok(Value::Int(i64::from(u32::from(*c))))),
                _ => None,
            },
            ("len", [x]) => match self.target(self.peek(x)?).ok()? {
                Value::List(xs) => Some(Ok(Value::Int(xs.len() as i64))),
                _ => None,
            },
            // A range's next number (`iter_next`), counted in place.
            ("Iter.next", [r]) => {
                let p = self.pointer_of(self.peek(r)?).ok()?;
                let Ok(Value::Iter(st)) = self.write(&p) else { return None };
                let IterState::Range { cur, stop, step } = &mut **st else { return None };
                let more = if *step > 0 { *cur < *stop } else { *cur > *stop };
                if !more {
                    return Some(Ok(Value::none()));
                }
                let v = *cur;
                match cur.checked_add(*step) {
                    Some(n) => *cur = n,
                    None => **st = IterState::Done,
                }
                Some(Ok(Value::some(Value::Int(v))))
            }
            ("list.push", [l, x]) => {
                let p = self.pointer_of(self.peek(l)?).ok()?;
                if !matches!(self.read(&p), Ok(Value::List(_))) || !self.evaluates(x) {
                    return None;
                }
                Some(self.operand_value(x).and_then(|x| match self.write(&p)? {
                    Value::List(xs) => {
                        xs.push(x);
                        Ok(Value::Unit)
                    }
                    other => Err(ice(format!("expected a list, found {other:?}"))),
                }))
            }
            ("list.pop", [l]) => {
                let p = self.pointer_of(self.peek(l)?).ok()?;
                match self.write(&p) {
                    Ok(Value::List(xs)) => Some(Ok(Value::option(xs.pop()))),
                    _ => None,
                }
            }
            ("list.is_empty", [l]) => {
                let p = self.pointer_of(self.peek(l)?).ok()?;
                match self.read(&p).ok()? {
                    Value::List(xs) => Some(Ok(Value::Bool(xs.is_empty()))),
                    _ => None,
                }
            }
            ("list.last", [l]) => {
                let p = self.pointer_of(self.peek(l)?).ok()?;
                let Value::List(xs) = self.read(&p).ok()? else { return None };
                let last = xs.len().checked_sub(1);
                Some(Ok(Value::option(
                    last.map(|i| Value::Ref(Rc::new(p.join(crate::value::Step::Index(i))))),
                )))
            }
            ("str.byte_len", [t]) => Some(Ok(Value::Int(self.peek_text(t)?.len() as i64))),
            ("str.to_string", [t]) => Some(Ok(Value::String(self.peek_text(t)?.to_string()))),
            ("str.byte_slice", [t, start, end]) => {
                let (Value::Int(start), Value::Int(end)) = (self.scalar(start)?, self.scalar(end)?) else {
                    return None;
                };
                let tag = match self.target(self.peek(t)?).ok()? {
                    Value::Str(_, tag) => *tag,
                    _ => 0,
                };
                let s = self.peek_text(t)?;
                let ok = 0 <= start && start <= end && end <= s.len() as i64;
                let (start, end) = (start as usize, end as usize);
                if !ok || !s.is_char_boundary(start) || !s.is_char_boundary(end) {
                    return None;
                }
                Some(Ok(Value::Str(Rc::from(&s[start..end]), tag)))
            }
            ("char.is_whitespace", [c]) => match self.scalar(c)? {
                Value::Char(c) => Some(Ok(Value::Bool(c.is_whitespace()))),
                _ => None,
            },
            ("char.from_code", [n]) => match self.scalar(n)? {
                Value::Int(n) => {
                    Some(Ok(Value::option(u32::try_from(n).ok().and_then(char::from_u32).map(Value::Char))))
                }
                _ => None,
            },
            ("range", [start, stop, step]) => {
                match (self.scalar(start)?, self.scalar(stop)?, self.scalar(step)?) {
                    (Value::Int(cur), Value::Int(stop), Value::Int(step)) if step != 0 => {
                        Some(Ok(Value::Iter(Box::new(IterState::Range { cur, stop, step }))))
                    }
                    _ => None,
                }
            }
            // Interpolation (lowering's `fmt.new`, then a `fmt.text` or
            // `fmt.display` per part): the parts pushed onto the `String`
            // being made.
            ("fmt.new", []) => Some(Ok(Value::String(String::new()))),
            ("fmt.text" | "fmt.display", [r, v, ..]) => {
                let p = self.pointer_of(self.peek(r)?).ok()?;
                let text = match (name, v) {
                    ("fmt.text", Operand::Const(sir::Const::Str(t))) => t.clone(),
                    ("fmt.text", _) => self.peek_text(v)?.to_string(),
                    _ => self.display_string(self.target(self.peek(v)?).ok()?),
                };
                match self.write(&p) {
                    Ok(Value::String(s)) => {
                        s.push_str(&text);
                        Some(Ok(Value::Unit))
                    }
                    _ => None,
                }
            }
            ("String.push", [s, t]) => {
                let p = self.pointer_of(self.peek(s)?).ok()?;
                let extra = match self.target(self.peek(t)?).ok()? {
                    Value::Str(t, _) => t.clone(),
                    _ => return None,
                };
                match self.write(&p) {
                    Ok(Value::String(s)) => {
                        s.push_str(&extra);
                        Some(Ok(Value::Unit))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The value of a copy of a number, `bool`, `char` or `()` outside
    /// the monitor, read in place; `None` for anything else, and for a
    /// read that fails, which [`Interp::operand`] then makes and reports.
    fn scalar(&self, o: &Operand) -> Option<Value> {
        if self.monitor.is_some() {
            return None;
        }
        let v = match o {
            Operand::Const(c) => return scalar_const(c),
            Operand::Copy(p) => self.read_fast(p)?.ok()?,
            Operand::Move(_) => return None,
        };
        match v {
            Value::Int(_)
            | Value::Bool(_)
            | Value::Char(_)
            | Value::Unit
            | Value::Float(_)
            | Value::F32(_) => Some(v.clone()),
            _ => None,
        }
    }

    /// An rvalue that only copies or combines such values, computed the
    /// way [`Interp::rvalue`] does, without its general machinery: `None`
    /// for every other rvalue, and where it would fail or overflow.
    fn scalar_rvalue(&self, r: &Rvalue) -> Option<Value> {
        match r {
            Rvalue::Use(o) => self.scalar(o),
            Rvalue::Binary(op, a, b) => {
                use sir::BinOp;
                let (a, b) = (self.scalar(a)?, self.scalar(b)?);
                let ordered = |o: std::cmp::Ordering| match op {
                    BinOp::Lt => Some(o.is_lt()),
                    BinOp::Le => Some(o.is_le()),
                    BinOp::Gt => Some(o.is_gt()),
                    BinOp::Ge => Some(o.is_ge()),
                    BinOp::Eq => Some(o.is_eq()),
                    BinOp::Ne => Some(o.is_ne()),
                    _ => None,
                };
                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => match op {
                        BinOp::Add => x.checked_add(y).map(Value::Int),
                        BinOp::Sub => x.checked_sub(y).map(Value::Int),
                        BinOp::Mul => x.checked_mul(y).map(Value::Int),
                        _ => ordered(x.cmp(&y)).map(Value::Bool),
                    },
                    (Value::Char(x), Value::Char(y)) => ordered(x.cmp(&y)).map(Value::Bool),
                    (Value::Bool(x), Value::Bool(y)) => ordered(x.cmp(&y)).map(Value::Bool),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn operand_value(&mut self, o: &'p Operand) -> Result<Value, Unwind> {
        match o {
            Operand::Const(c) => Ok(self.const_value(c)),
            Operand::Copy(p) => {
                if let Some(v) = self.read_fast(p) {
                    let v = v?;
                    if v.is_moved() {
                        return Err(panic(msg::PANIC_MOVED_VALUE, &[]));
                    }
                    return Ok(v.clone());
                }
                let ptr = self.resolve(p)?;
                self.mon_access(&ptr, crate::monitor::Op::Read)?;
                let v = self.read(&ptr)?;
                if v.is_moved() {
                    return Err(self.moved_at(&ptr));
                }
                Ok(v.clone())
            }
            Operand::Move(p) => {
                // A whole local of the top frame, outside the monitor.
                if p.proj.is_empty()
                    && self.monitor.is_none()
                    && let Some(slot) =
                        self.frames.last_mut().and_then(|f| f.locals.get_mut(p.local.0 as usize))
                {
                    let v = std::mem::replace(slot, Value::Moved);
                    if v.is_moved() {
                        return Err(panic(msg::PANIC_MOVED_VALUE, &[]));
                    }
                    return Ok(v);
                }
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
                // A reborrow through a reference of the top frame, outside
                // the monitor, shares the reference's pointer.
                if self.monitor.is_none()
                    && let [Proj::Deref] = p.proj.as_slice()
                    && let Some(v) = self.frames.last().and_then(|f| f.locals.get(p.local.0 as usize))
                {
                    match v {
                        Value::Ref(q) => return Ok(Value::Ref(q.clone())),
                        // Reborrowing a `ref dyn` made from a `ref T` keeps `T`.
                        Value::Dyn(d) if matches!(d.ty, sir::Type::Ref(..)) => {
                            if let Value::Ref(q) = &d.value {
                                return Ok(Value::Dyn(Box::new(crate::value::DynVal {
                                    ty: d.ty.clone(),
                                    value: Value::Ref(q.clone()),
                                })));
                            }
                        }
                        _ => {}
                    }
                }
                let mut ptr = self.resolve(p)?;
                if self.monitoring() {
                    // Borrowing a moved-from place is a use after move.
                    if self.read(&ptr)?.is_moved() {
                        return Err(self.moved_at(&ptr));
                    }
                    ptr.tag = self.mon_borrow(&ptr, *m, !self.dest_is_temp())?;
                }
                let r = Value::Ref(Rc::new(ptr));
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
                if let Some(v) = self.read_fast(p) {
                    return match self.target(v?)? {
                        Value::Variant(a) => Ok(Value::Int(a.variant as i64)),
                        other => Err(ice(format!("discriminant of {other:?}"))),
                    };
                }
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
pub(crate) fn display_fn_name(name: &str, prog: &sir::Program) -> String {
    if let Some(i) = name.strip_prefix("test#").and_then(|n| n.parse::<usize>().ok())
        && let Some((t, _)) = prog.tests.get(i)
    {
        return format!("test {}", sisu_sir::write::quote(t));
    }
    if let Some(owner) = name.strip_suffix(".__check_invariant") {
        return format!("the invariant of {owner}");
    }
    match name.split_once("#closure") {
        Some((owner, _)) => format!("closure in {}", display_fn_name(owner, prog)),
        None => name.to_string(),
    }
}

impl<'p> Interp<'p, '_> {
    /// A constant of the program as a value; a string or a function name
    /// is made once and shared by every evaluation of the constant.
    fn const_value(&mut self, c: &'p sir::Const) -> Value {
        match c {
            sir::Const::Str(s) => {
                let key = s as *const String as usize;
                let text = self.const_texts.entry(key).or_insert_with(|| Rc::from(s.as_str())).clone();
                Value::Str(text, 0)
            }
            sir::Const::Fn(n) => {
                let key = n as *const String as usize;
                Value::Fn(self.const_texts.entry(key).or_insert_with(|| Rc::from(n.as_str())).clone())
            }
            other => const_value(other),
        }
    }
}

/// A constant number, `bool`, `char` or `()`.
fn scalar_const(c: &sir::Const) -> Option<Value> {
    match c {
        sir::Const::Int(n) => Some(Value::Int(*n)),
        sir::Const::Float(f) => Some(Value::Float(*f)),
        sir::Const::Bool(b) => Some(Value::Bool(*b)),
        sir::Const::Char(c) => Some(Value::Char(*c)),
        sir::Const::Unit => Some(Value::Unit),
        sir::Const::Str(_) | sir::Const::Fn(_) => None,
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
