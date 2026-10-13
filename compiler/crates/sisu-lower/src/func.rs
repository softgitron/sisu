//! Lowering one function: the block builder, scopes and drops.
//!
//! Each block's unwind edge is computed when the block starts, from the
//! values that need drop and are live at that point; the cleanup chain for
//! a live set is built once and shared. A statement that makes a value
//! live (initialises a local that needs drop) or dead (drops it) ends its
//! block, so every block's unwind edge is exact. A function is lowered
//! twice: the first pass finds the locals that are moved as a whole, and
//! the second gives each of them a drop flag.

use crate::Ctx;
use sisu_resolve::Ty;
use sisu_sir as sir;
use sisu_sir::{BlockId, Place, Statement, StatementKind, Terminator};
use sisu_typeck::thir;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DropEntry {
    Local(sir::LocalId, Option<sir::LocalId>),
    /// A `defer` block, by index into `FnLower::defers`.
    Defer(usize),
}

pub(crate) struct Scope {
    pub entries: Vec<DropEntry>,
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
    pub closure: Option<thir::ClosureId>,
    pub name: String,
    pub locals: Vec<sir::LocalDecl>,
    pub map: HashMap<thir::LocalId, sir::LocalId>,
    pub alias: HashMap<thir::LocalId, Place>,
    pub env: HashMap<thir::LocalId, u32>,
    blocks: Vec<BlockBuild>,
    cur: usize,
    pub scopes: Vec<Scope>,
    pub flags: HashMap<thir::LocalId, sir::LocalId>,
    needs_flag: HashSet<thir::LocalId>,
    pub moved: HashSet<thir::LocalId>,
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
    let kind = match &body.kind {
        thir::BodyKind::Fn => match ctx.prog.fn_info(body.def).map(|f| &f.kind) {
            Some(sisu_resolve::FnKind::Test(_)) => sir::FnKind::Test,
            Some(sisu_resolve::FnKind::Drop) => sir::FnKind::Drop,
            _ => sir::FnKind::Normal,
        },
        _ => sir::FnKind::Value,
    };
    out.push(lower_twice(ctx, body, None, &name, kind, source));
    for i in 0..body.closures.len() {
        let id = thir::ClosureId(i as u32);
        let cname = closure_name(&name, id);
        out.push(lower_twice(ctx, body, Some(id), &cname, sir::FnKind::Closure, source));
    }
    out
}

pub(crate) fn closure_name(owner: &str, id: thir::ClosureId) -> String {
    format!("{owner}#closure{}", id.0)
}

fn lower_twice(
    ctx: &Ctx,
    body: &thir::Body,
    closure: Option<thir::ClosureId>,
    name: &str,
    kind: sir::FnKind,
    source: u32,
) -> sir::Function {
    let first = FnLower::new(ctx, body, closure, name, HashSet::new()).run(kind, source);
    FnLower::new(ctx, body, closure, name, first.1).run(kind, source).0
}

impl<'c, 'a> FnLower<'c, 'a> {
    fn new(
        ctx: &'c Ctx<'a>,
        body: &'a thir::Body,
        closure: Option<thir::ClosureId>,
        name: &str,
        needs_flag: HashSet<thir::LocalId>,
    ) -> FnLower<'c, 'a> {
        FnLower {
            ctx,
            body,
            closure,
            name: name.to_string(),
            locals: Vec::new(),
            map: HashMap::new(),
            alias: HashMap::new(),
            env: HashMap::new(),
            blocks: Vec::new(),
            cur: 0,
            scopes: Vec::new(),
            flags: HashMap::new(),
            needs_flag,
            moved: HashSet::new(),
            cleanups: HashMap::new(),
            resume_block: None,
            override_live: None,
            in_cleanup: false,
            loops: Vec::new(),
            defers: Vec::new(),
            unit_temp: None,
        }
    }

    fn run(mut self, kind: sir::FnKind, source: u32) -> (sir::Function, HashSet<thir::LocalId>) {
        let body = self.body;
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
        if let Some(c) = self.closure {
            let cl = &body.closures[c.0 as usize];
            let env_tys: Vec<sir::Type> = cl
                .captures
                .iter()
                .map(|cap| {
                    sir::Type::Ref(Box::new(self.ctx.ty(&body.locals[cap.local.0 as usize].ty)), cap.mutable)
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
        self.scopes.push(Scope { entries: Vec::new() });
        self.switch_to(entry);
        for p in &params {
            let ty = body.locals[p.0 as usize].ty.clone();
            let name = body.locals[p.0 as usize].name.clone();
            let l = self.add_local(self.ctx.ty(&ty), Some(name));
            self.map.insert(*p, l);
            param_count += 1;
        }
        for p in &params {
            let ty = body.locals[p.0 as usize].ty.clone();
            if self.needs_drop(&ty) {
                let l = self.map[p];
                let flag = self.flag_init(*p);
                self.register(DropEntry::Local(l, flag));
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
            self.emit_exit(0);
            self.terminate(Terminator::Return, None);
        }
        let blocks = self.finish_blocks();
        let f =
            sir::Function { name: self.name.clone(), kind, param_count, locals: self.locals, blocks, source };
        (f, self.moved)
    }

    // ---------------------------------------------------------------- locals

    pub fn add_local(&mut self, ty: sir::Type, name: Option<String>) -> sir::LocalId {
        let id = sir::LocalId(self.locals.len() as u32);
        self.locals.push(sir::LocalDecl { ty, name });
        id
    }

    pub fn temp(&mut self, ty: &Ty) -> sir::LocalId {
        let t = self.ctx.ty(ty);
        self.add_local(t, None)
    }

    pub fn temp_sir(&mut self, ty: sir::Type) -> sir::LocalId {
        self.add_local(ty, None)
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
        self.map.insert(l, s);
        s
    }

    pub fn needs_drop(&self, ty: &Ty) -> bool {
        !self.ctx.prog.is_copy(ty) && !matches!(ty, Ty::Ref(..))
    }

    /// The drop flag of a local that is moved as a whole somewhere, set
    /// to true now.
    pub fn flag_init(&mut self, l: thir::LocalId) -> Option<sir::LocalId> {
        if !self.needs_flag.contains(&l) {
            return None;
        }
        let f = match self.flags.get(&l) {
            Some(&f) => f,
            None => {
                let f = self.add_local(sir::Type::Bool, Some("flag".into()));
                self.flags.insert(l, f);
                f
            }
        };
        self.assign(Place::local(f), sir::Rvalue::Use(sir::Operand::Const(sir::Const::Bool(true))), None);
        Some(f)
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
        self.scopes.push(Scope { entries: Vec::new() });
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
            self.emit_drop(e);
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

    fn emit_drop(&mut self, e: &DropEntry) {
        match e {
            DropEntry::Local(l, flag) => {
                self.emit(StatementKind::Drop { place: Place::local(*l), flag: *flag }, None)
            }
            DropEntry::Defer(i) => {
                let block = self.defers[*i].clone();
                self.push_scope();
                self.block_stmts(&block);
                self.pop_scope();
            }
        }
    }

    /// Pops the innermost scope, dropping its values in reverse order.
    pub fn pop_scope(&mut self) {
        while let Some(e) = self.scopes.last_mut().and_then(|s| s.entries.pop()) {
            if self.terminated() {
                continue;
            }
            if self.in_cleanup {
                self.emit_drop(&e);
            } else {
                self.split();
                self.emit_drop(&e);
                if matches!(e, DropEntry::Local(..)) {
                    self.split();
                }
            }
        }
        self.scopes.pop();
    }

    /// Drops every value of the scopes from `depth` up, innermost first,
    /// without popping them: the path to a `return`, `break` or `continue`.
    pub fn emit_exit(&mut self, depth: usize) {
        if self.terminated() {
            return;
        }
        let base: Vec<DropEntry> = self.scopes[..depth.min(self.scopes.len())]
            .iter()
            .flat_map(|s| s.entries.iter().cloned())
            .collect();
        let leaving: Vec<DropEntry> = self.scopes[depth.min(self.scopes.len())..]
            .iter()
            .flat_map(|s| s.entries.iter().cloned())
            .collect();
        if leaving.is_empty() {
            return;
        }
        let mut remaining: Vec<DropEntry> = base.into_iter().chain(leaving.iter().cloned()).collect();
        let saved = self.override_live.take();
        for e in leaving.iter().rev() {
            remaining.pop();
            self.override_live = Some(remaining.clone());
            if self.in_cleanup {
                self.emit_drop(e);
            } else {
                self.split();
                self.emit_drop(e);
            }
        }
        self.override_live = saved;
    }

    /// A block that drops the scopes from `depth` up and jumps to `target`.
    pub fn edge_to(&mut self, target: BlockId, depth: usize) -> BlockId {
        let leaving = self.scopes[depth.min(self.scopes.len())..].iter().any(|s| !s.entries.is_empty());
        if !leaving {
            return target;
        }
        let saved = self.cur;
        let b = self.new_block();
        self.switch_to(b);
        self.emit_exit(depth);
        self.goto(target);
        self.cur = saved;
        b
    }

    // ---------------------------------------------------------------- finishing

    fn finish_blocks(&mut self) -> Vec<sir::BasicBlock> {
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
