//! Lowering one function: the block builder, scopes and drops.
//!
//! Each block's unwind edge is computed when the block starts, from the
//! values that need drop and are registered at that point; the cleanup
//! chain for a set of values is built once and shared. A statement that
//! drops a value ends its block, so every block's unwind edge is exact.
//!
//! Drops are emitted unconditionally: every value that needs drop is
//! dropped at the end of its scope, on every exit, whether or not it was
//! moved. The ownership checker's initialisation dataflow (`sisu-borrowck`)
//! then removes the drops of moved values, guards the drops of values that
//! may have been moved with a flag, and splits the drop of a partly moved
//! value into drops of what remains. Every local that leaves a scope also
//! gets a `dead` statement; [`finish_blocks`](FnLower) keeps only those of
//! locals that are borrowed somewhere, which are the ones the checker and
//! the monitor need.

use crate::Ctx;
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Place, Statement, StatementKind, Terminator};
use sisu_typeck::thir;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DropEntry {
    Local(sir::LocalId),
    /// A `defer` block, by index into `FnLower::defers`.
    Defer(usize),
}

pub(crate) struct Scope {
    pub entries: Vec<DropEntry>,
    /// Every local whose storage ends with the scope, in creation order.
    pub locals: Vec<sir::LocalId>,
    /// Where the scope ends: what a drop at its end points at.
    pub end: Option<sir::Loc>,
}

pub(crate) struct LoopCtx {
    pub break_bb: BlockId,
    pub continue_bb: BlockId,
    /// Scopes at and above this index are left by `break` and `continue`.
    pub depth: usize,
}

struct BlockBuild {
    stmts: Vec<Statement>,
    term: Option<Terminator>,
    unwind: Option<BlockId>,
    cleanup: bool,
    started: bool,
    loc: Option<sir::Loc>,
}

pub(crate) struct FnLower<'c, 'a> {
    pub ctx: &'c Ctx<'a>,
    pub body: &'a thir::Body,
    /// The generic parameters of the function (a closure has its owner's).
    pub type_params: Vec<String>,
    /// The bounds in scope, for associated types and trait calls.
    pub bounds: Vec<sisu_resolve::Predicate>,
    pub closure: Option<thir::ClosureId>,
    pub name: String,
    pub locals: Vec<sir::LocalDecl>,
    pub map: HashMap<thir::LocalId, sir::LocalId>,
    pub alias: HashMap<thir::LocalId, Place>,
    pub env: HashMap<thir::LocalId, u32>,
    blocks: Vec<BlockBuild>,
    cur: usize,
    pub scopes: Vec<Scope>,
    cleanups: HashMap<Vec<DropEntry>, BlockId>,
    resume_block: Option<BlockId>,
    override_live: Option<Vec<DropEntry>>,
    in_cleanup: bool,
    pub loops: Vec<LoopCtx>,
    pub defers: Vec<thir::Block>,
    pub unit_temp: Option<sir::LocalId>,
}

/// Lowers a body and its closures to SIR functions.
pub(crate) fn lower_body(ctx: &mut Ctx, body: &thir::Body) -> Vec<sir::Function> {
    let name = ctx.body_name(body);
    let source = ctx.source_index(body.span.file);
    let ctx: &Ctx = ctx;
    let mut out = Vec::new();
    let fn_info = match &body.kind {
        thir::BodyKind::Fn => ctx.prog.fn_info(body.def),
        _ => None,
    };
    let kind = match fn_info.map(|f| &f.kind) {
        Some(sisu_resolve::FnKind::Test(_)) => sir::FnKind::Test,
        Some(sisu_resolve::FnKind::Drop) => sir::FnKind::Drop,
        Some(_) => sir::FnKind::Normal,
        None => sir::FnKind::Value,
    };
    let (pure, from, inner, own) = match fn_info {
        Some(f) => {
            let b = ctx.prog.result_borrows_of(f);
            let own: Vec<u32> = ctx
                .prog
                .sig_params(f)
                .iter()
                .enumerate()
                .filter(|(_, p)| p.mode == sisu_syntax::ast::Mode::Own)
                .map(|(i, _)| i as u32)
                .collect();
            (f.pure, b.deep, b.inner, own)
        }
        None => (true, Vec::new(), Vec::new(), Vec::new()),
    };
    let generics_of = match &body.kind {
        thir::BodyKind::Fn => ctx.prog.fn_info(body.def),
        _ => None,
    };
    let type_params: Vec<String> =
        generics_of.map(|f| f.generics.iter().map(|g| g.name.clone()).collect()).unwrap_or_default();
    let env = generics_of.map(|f| ctx.prog.elaborate_env(&f.predicates)).unwrap_or_default();
    let mut main = FnLower::new(ctx, body, None, &name);
    main.type_params = type_params.clone();
    main.bounds = env.clone();
    out.push(main.run(kind, source, pure, (from, inner), own));
    for (i, cl) in body.closures.iter().enumerate() {
        let id = thir::ClosureId(i as u32);
        if cl.comptime {
            // `comptime e`: a function of nothing, which the compiler runs
            // with the item's budget.
            let budget = ctx.prog.comptime_budgets.get(&body.def).copied().unwrap_or(DEFAULT_COMPTIME_BUDGET);
            // Its value cannot depend on the owner's type arguments (E0600).
            let mut fl = FnLower::new(ctx, body, Some(id), &comptime_name(&name, id));
            fl.bounds = env.clone();
            let pure = matches!(&cl.fn_ty, Ty::Fn(f) if f.pure);
            out.push(fl.run(
                sir::FnKind::Comptime(budget),
                source,
                pure,
                (Vec::new(), Vec::new()),
                Vec::new(),
            ));
            continue;
        }
        let cname = closure_name(&name, id);
        let pure = matches!(&cl.fn_ty, Ty::Fn(f) if f.pure);
        // A closure's result may borrow from any argument, its environment
        // (argument 0) included: function types carry no `from` clause.
        let from: Vec<u32> = (0..cl.params.len() as u32 + 1).collect();
        let mut fl = FnLower::new(ctx, body, Some(id), &cname);
        fl.type_params = type_params.clone();
        fl.bounds = env.clone();
        out.push(fl.run(sir::FnKind::Closure, source, pure, (from, Vec::new()), Vec::new()));
    }
    out
}

pub(crate) fn closure_name(owner: &str, id: thir::ClosureId) -> String {
    format!("{owner}#closure{}", id.0)
}

/// The steps a `comptime` expression may take without
/// `@comptime_budget(steps=N)` on its item (08 leaves the number open).
pub const DEFAULT_COMPTIME_BUDGET: u64 = 1_000_000;

pub(crate) fn comptime_name(owner: &str, id: thir::ClosureId) -> String {
    format!("{owner}#comptime{}", id.0)
}

impl<'c, 'a> FnLower<'c, 'a> {
    fn new(ctx: &'c Ctx<'a>, body: &'a thir::Body, closure: Option<thir::ClosureId>, name: &str) -> Self {
        FnLower {
            ctx,
            body,
            type_params: Vec::new(),
            bounds: Vec::new(),
            closure,
            name: name.to_string(),
            locals: Vec::new(),
            map: HashMap::new(),
            alias: HashMap::new(),
            env: HashMap::new(),
            blocks: Vec::new(),
            cur: 0,
            scopes: Vec::new(),
            cleanups: HashMap::new(),
            resume_block: None,
            override_live: None,
            in_cleanup: false,
            loops: Vec::new(),
            defers: Vec::new(),
            unit_temp: None,
        }
    }

    /// Lowers the body; `borrows` are the arguments the result may borrow
    /// from, deeply and through their values only.
    fn run(
        mut self,
        kind: sir::FnKind,
        source: u32,
        pure: bool,
        borrows: (Vec<u32>, Vec<u32>),
        own: Vec<u32>,
    ) -> sir::Function {
        let (from, inner) = borrows;
        let body = self.body;
        // Where the function is declared: its name, or the closure.
        let loc = match self.closure {
            Some(c) => self.ctx.loc(body.closures[c.0 as usize].span),
            None if matches!(body.kind, thir::BodyKind::Fn) => self.ctx.loc(self.ctx.prog.def(body.def).span),
            None => None,
        };
        let (ret, block, params) = match self.closure {
            None => (body.ret.clone(), &body.block, body.params.clone()),
            Some(c) => {
                let cl = &body.closures[c.0 as usize];
                (cl.ret.clone(), &cl.body, cl.params.clone())
            }
        };
        self.add_local(self.ctx.ty(&ret), None);
        // Parameters.
        let mut param_count = 0;
        if let Some(c) = self.closure.filter(|c| !body.closures[c.0 as usize].comptime) {
            let cl = &body.closures[c.0 as usize];
            let env_tys: Vec<sir::Type> = cl
                .captures
                .iter()
                .map(|cap| {
                    let t = self.ctx.ty(&body.locals[cap.local.0 as usize].ty);
                    if cl.own { t } else { sir::Type::Ref(Box::new(t), cap.mutable) }
                })
                .collect();
            self.add_local(
                if env_tys.is_empty() { sir::Type::Unit } else { sir::Type::Tuple(env_tys) },
                Some("env".into()),
            );
            for (i, cap) in cl.captures.iter().enumerate() {
                self.env.insert(cap.local, i as u32);
            }
            param_count += 1;
        }
        let entry = self.new_block();
        self.scopes.push(Scope { entries: Vec::new(), locals: Vec::new(), end: self.block_end(block) });
        self.switch_to(entry);
        for p in &params {
            let decl = &body.locals[p.0 as usize];
            let l = self.add_local(self.ctx.ty(&decl.ty), Some(decl.name.clone()));
            self.locals[l.0 as usize].loc = self.ctx.loc(decl.span);
            self.map.insert(*p, l);
            param_count += 1;
        }
        for p in &params {
            let ty = body.locals[p.0 as usize].ty.clone();
            if self.needs_drop(&ty) {
                let l = self.map[p];
                self.register(DropEntry::Local(l));
            }
        }
        if !self.scopes[0].entries.is_empty() {
            self.split();
        }
        self.block_stmts(block);
        if !self.terminated() {
            // Falling off the end: only unit functions do (the checker
            // rejects the rest).
            self.assign(
                Place::local(sir::LocalId(0)),
                sir::Rvalue::Use(sir::Operand::Const(sir::Const::Unit)),
                None,
            );
            let end = self.scopes.first().and_then(|s| s.end);
            self.emit_exit(0, end);
            self.terminate(Terminator::Return, None);
        }
        let blocks = self.finish_blocks();
        sir::Function {
            name: self.name.clone(),
            type_params: self.type_params.clone(),
            kind,
            param_count,
            pure,
            from,
            inner,
            own,
            locals: self.locals,
            blocks,
            source,
            loc,
        }
    }

    /// The end of a block's last statement: where its scope ends.
    pub fn block_end(&self, b: &thir::Block) -> Option<sir::Loc> {
        let span = b.stmts.last().map(stmt_span).unwrap_or(b.span);
        self.ctx.end_loc(span)
    }

    // ---------------------------------------------------------------- locals

    pub fn add_local(&mut self, ty: sir::Type, name: Option<String>) -> sir::LocalId {
        let id = sir::LocalId(self.locals.len() as u32);
        self.locals.push(sir::LocalDecl::new(ty, name));
        id
    }

    /// A temporary whose storage ends with the innermost scope.
    pub fn temp(&mut self, ty: &Ty) -> sir::LocalId {
        let t = self.ctx.ty(ty);
        self.temp_sir(t)
    }

    pub fn temp_sir(&mut self, ty: sir::Type) -> sir::LocalId {
        let l = self.add_local(ty, None);
        self.scope_local(0, l);
        l
    }

    /// The scratch destination of calls whose `()` result is unused.
    pub fn unit(&mut self) -> Place {
        let l = match self.unit_temp {
            Some(l) => l,
            None => {
                let l = self.add_local(sir::Type::Unit, None);
                self.unit_temp = Some(l);
                l
            }
        };
        Place::local(l)
    }

    /// The SIR local of a THIR local, created on first use.
    pub fn local_for(&mut self, l: thir::LocalId) -> sir::LocalId {
        if let Some(&s) = self.map.get(&l) {
            return s;
        }
        let decl = &self.body.locals[l.0 as usize];
        let ty = self.ctx.ty(&decl.ty);
        let name = if decl.name.is_empty() || decl.name == "?" { None } else { Some(decl.name.clone()) };
        let s = self.add_local(ty, name);
        self.locals[s.0 as usize].loc = self.ctx.loc(decl.span);
        self.locals[s.0 as usize].once = decl.once;
        self.map.insert(l, s);
        s
    }

    /// Records that `l`'s storage ends with the scope `below` levels under
    /// the innermost.
    pub fn scope_local(&mut self, below: usize, l: sir::LocalId) {
        let len = self.scopes.len();
        if let Some(s) = self.scopes.get_mut(len.saturating_sub(1 + below)) {
            s.locals.push(l);
        }
    }

    pub fn needs_drop(&self, ty: &Ty) -> bool {
        !self.ctx.prog.is_copy_in(ty, &self.bounds) && !matches!(ty, Ty::Ref(..))
    }

    // ---------------------------------------------------------------- blocks

    pub fn new_block(&mut self) -> BlockId {
        self.blocks.push(BlockBuild {
            stmts: Vec::new(),
            term: None,
            unwind: None,
            cleanup: false,
            started: false,
            loc: None,
        });
        BlockId(self.blocks.len() as u32 - 1)
    }

    pub fn switch_to(&mut self, b: BlockId) {
        self.cur = b.0 as usize;
        if !self.blocks[self.cur].started {
            self.blocks[self.cur].started = true;
            self.blocks[self.cur].cleanup = self.in_cleanup;
            self.blocks[self.cur].unwind = if self.in_cleanup { None } else { self.cleanup_for_live() };
        }
    }

    pub fn terminated(&self) -> bool {
        self.blocks[self.cur].term.is_some()
    }

    pub fn emit(&mut self, kind: StatementKind, loc: Option<sir::Loc>) {
        if self.terminated() {
            // Code after a diverging statement goes to a fresh, unreachable block.
            let b = self.new_block();
            self.switch_to(b);
        }
        self.blocks[self.cur].stmts.push(Statement { kind, loc });
    }

    pub fn assign(&mut self, p: Place, r: sir::Rvalue, loc: Option<sir::Loc>) {
        self.emit(StatementKind::Assign(p, r), loc);
    }

    pub fn terminate(&mut self, t: Terminator, loc: Option<sir::Loc>) {
        if self.blocks[self.cur].term.is_none() {
            self.blocks[self.cur].term = Some(t);
            self.blocks[self.cur].loc = loc;
        }
    }

    pub fn goto(&mut self, b: BlockId) {
        self.terminate(Terminator::Goto(b), None);
    }

    /// Ends the current block with a jump to a fresh one.
    pub fn split(&mut self) {
        let b = self.new_block();
        self.goto(b);
        self.switch_to(b);
    }

    // ---------------------------------------------------------------- scopes and drops

    pub fn push_scope(&mut self) {
        let end = self.scopes.last().and_then(|s| s.end);
        self.scopes.push(Scope { entries: Vec::new(), locals: Vec::new(), end });
    }

    /// A scope that ends at `end` (a block's last statement, a statement).
    pub fn push_scope_at(&mut self, end: Option<sir::Loc>) {
        self.scopes.push(Scope { entries: Vec::new(), locals: Vec::new(), end });
    }

    /// Registers a value to drop in the innermost scope; the next
    /// statement starts a block whose unwind edge includes it.
    pub fn register(&mut self, e: DropEntry) {
        if let Some(s) = self.scopes.last_mut() {
            s.entries.push(e);
        }
    }

    /// Registers in the scope `n` levels below the innermost.
    pub fn register_below(&mut self, n: usize, e: DropEntry) {
        let len = self.scopes.len();
        if let Some(s) = self.scopes.get_mut(len.saturating_sub(1 + n)) {
            s.entries.push(e);
        }
    }

    fn live(&self) -> Vec<DropEntry> {
        if let Some(o) = &self.override_live {
            return o.clone();
        }
        self.scopes.iter().flat_map(|s| s.entries.iter().cloned()).collect()
    }

    fn cleanup_for_live(&mut self) -> Option<BlockId> {
        let live = self.live();
        if live.is_empty() { None } else { Some(self.cleanup_chain(&live)) }
    }

    fn cleanup_chain(&mut self, entries: &[DropEntry]) -> BlockId {
        if let Some(&b) = self.cleanups.get(entries) {
            return b;
        }
        let next =
            if entries.len() > 1 { self.cleanup_chain(&entries[..entries.len() - 1]) } else { self.resume() };
        let saved = (self.cur, self.in_cleanup, self.override_live.take());
        self.in_cleanup = true;
        let b = self.new_block();
        self.switch_to(b);
        if let Some(e) = entries.last() {
            self.emit_drop(e, None);
        }
        self.goto(next);
        self.cur = saved.0;
        self.in_cleanup = saved.1;
        self.override_live = saved.2;
        self.cleanups.insert(entries.to_vec(), b);
        b
    }

    fn resume(&mut self) -> BlockId {
        if let Some(b) = self.resume_block {
            return b;
        }
        let saved = (self.cur, self.in_cleanup);
        self.in_cleanup = true;
        let b = self.new_block();
        self.switch_to(b);
        self.terminate(Terminator::Resume, None);
        self.cur = saved.0;
        self.in_cleanup = saved.1;
        self.resume_block = Some(b);
        b
    }

    fn emit_drop(&mut self, e: &DropEntry, loc: Option<sir::Loc>) {
        match e {
            DropEntry::Local(l) => {
                self.emit(StatementKind::Drop { place: Place::local(*l), flag: None }, loc)
            }
            DropEntry::Defer(i) => {
                let block = self.defers[*i].clone();
                self.push_scope();
                self.block_stmts(&block);
                self.pop_scope();
            }
        }
    }

    /// Pops the innermost scope, dropping its values in reverse order and
    /// ending the storage of its locals.
    pub fn pop_scope(&mut self) {
        let end = self.scopes.last().and_then(|s| s.end);
        while let Some(e) = self.scopes.last_mut().and_then(|s| s.entries.pop()) {
            if self.terminated() {
                continue;
            }
            if self.in_cleanup {
                self.emit_drop(&e, None);
            } else {
                self.split();
                self.emit_drop(&e, end);
                if matches!(e, DropEntry::Local(..)) {
                    self.split();
                }
            }
        }
        let locals = self.scopes.pop().map(|s| s.locals).unwrap_or_default();
        if !self.terminated() && !self.in_cleanup {
            for l in locals.into_iter().rev() {
                self.emit(StatementKind::StorageDead(l), end);
            }
        }
    }

    /// Drops every value of the scopes from `depth` up, innermost first,
    /// without popping them: the path to a `return`, `break` or `continue`
    /// written at `at`. Leaving for a loop's `break` or `continue` also ends
    /// the storage of the scopes' locals; `return` leaves the frame.
    pub fn emit_exit(&mut self, depth: usize, at: Option<sir::Loc>) {
        if self.terminated() {
            return;
        }
        let depth = depth.min(self.scopes.len());
        let base: Vec<DropEntry> =
            self.scopes[..depth].iter().flat_map(|s| s.entries.iter().cloned()).collect();
        let leaving: Vec<DropEntry> =
            self.scopes[depth..].iter().flat_map(|s| s.entries.iter().cloned()).collect();
        let dead: Vec<sir::LocalId> = if depth == 0 {
            Vec::new()
        } else {
            self.scopes[depth..].iter().flat_map(|s| s.locals.iter().copied()).collect()
        };
        if !leaving.is_empty() {
            let mut remaining: Vec<DropEntry> = base.into_iter().chain(leaving.iter().cloned()).collect();
            let saved = self.override_live.take();
            for e in leaving.iter().rev() {
                remaining.pop();
                self.override_live = Some(remaining.clone());
                if self.in_cleanup {
                    self.emit_drop(e, None);
                } else {
                    self.split();
                    self.emit_drop(e, at);
                }
            }
            self.override_live = saved;
        }
        if !self.in_cleanup && !dead.is_empty() {
            if self.blocks[self.cur]
                .stmts
                .last()
                .is_some_and(|s| matches!(s.kind, StatementKind::Drop { .. }))
            {
                self.split();
            }
            for l in dead.into_iter().rev() {
                self.emit(StatementKind::StorageDead(l), at);
            }
        }
    }

    /// A block that drops the scopes from `depth` up and jumps to `target`.
    pub fn edge_to(&mut self, target: BlockId, depth: usize) -> BlockId {
        let leaving = self.scopes[depth.min(self.scopes.len())..]
            .iter()
            .any(|s| !s.entries.is_empty() || !s.locals.is_empty());
        if !leaving {
            return target;
        }
        let saved = self.cur;
        let b = self.new_block();
        self.switch_to(b);
        self.emit_exit(depth, None);
        self.goto(target);
        self.cur = saved;
        b
    }

    // ---------------------------------------------------------------- finishing

    fn finish_blocks(&mut self) -> Vec<sir::BasicBlock> {
        // Keep the `dead` statements of borrowed locals only.
        let mut borrowed: HashSet<sir::LocalId> = HashSet::new();
        for bb in &self.blocks {
            for s in &bb.stmts {
                if let StatementKind::Assign(_, sir::Rvalue::Ref(p, _) | sir::Rvalue::StrView(p)) = &s.kind {
                    borrowed.insert(p.local);
                }
            }
        }
        for bb in &mut self.blocks {
            bb.stmts.retain(|s| match s.kind {
                StatementKind::StorageDead(l) => borrowed.contains(&l),
                _ => true,
            });
        }
        // Drop blocks no edge reaches, and renumber.
        let n = self.blocks.len();
        let mut reach = vec![false; n];
        let mut work = vec![0usize];
        while let Some(b) = work.pop() {
            if b >= n || reach[b] {
                continue;
            }
            reach[b] = true;
            let bb = &self.blocks[b];
            if let Some(u) = bb.unwind {
                work.push(u.0 as usize);
            }
            match &bb.term {
                Some(Terminator::Goto(t)) => work.push(t.0 as usize),
                Some(Terminator::Switch { cases, default, .. }) => {
                    work.extend(cases.iter().map(|(_, t)| t.0 as usize));
                    work.push(default.0 as usize);
                }
                _ => {}
            }
        }
        let mut renum = vec![0u32; n];
        let mut next = 0u32;
        for (i, r) in reach.iter().enumerate() {
            if *r {
                renum[i] = next;
                next += 1;
            }
        }
        let fix = |b: BlockId| BlockId(renum[b.0 as usize]);
        let mut out = Vec::new();
        for (i, bb) in self.blocks.drain(..).enumerate() {
            if !reach[i] {
                continue;
            }
            let term = match bb.term.unwrap_or(Terminator::Unreachable) {
                Terminator::Goto(t) => Terminator::Goto(fix(t)),
                Terminator::Switch { value, cases, default } => Terminator::Switch {
                    value,
                    cases: cases.into_iter().map(|(v, t)| (v, fix(t))).collect(),
                    default: fix(default),
                },
                t => t,
            };
            out.push(sir::BasicBlock {
                stmts: bb.stmts,
                term,
                unwind: bb.unwind.map(fix),
                cleanup: bb.cleanup,
                loc: bb.loc,
            });
        }
        out
    }
}

/// The span of a statement.
pub(crate) fn stmt_span(s: &thir::Stmt) -> sisu_diag::Span {
    use thir::Stmt::*;
    match s {
        Let { span, .. }
        | Assign { span, .. }
        | AssignTuple { span, .. }
        | Discard { span, .. }
        | AugAssign { span, .. }
        | Return { span, .. }
        | If { span, .. }
        | While { span, .. }
        | For { span, .. }
        | Match { span, .. }
        | Assert { span, .. }
        | Defer { span, .. } => *span,
        Expr(e) => e.span,
        Break(span) | Continue(span) | Error(span) => *span,
        Block(b) => b.span,
    }
}
