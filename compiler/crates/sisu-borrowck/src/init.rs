//! Initialisation and moves: a forward dataflow over move paths.
//!
//! Each path has two bits: *may be initialised* (`mi`) and *may be
//! uninitialised* (`mu`), joined by union. Initialising a path sets `mi`
//! and clears `mu` for it and every path below it; moving, dropping or
//! ending the storage of a path does the opposite. A path is definitely
//! initialised where `mu` is clear for it and everything below it, and
//! definitely uninitialised where `mi` is clear likewise.
//!
//! A switch on a discriminant teaches the dataflow which variant is
//! active on each edge: the payload paths of the other variants hold
//! nothing there, so both bits are cleared ("vacuous"), which lets the
//! drop elaboration remove the drop of a payload that cannot exist.
//!
//! Unwind edges carry the state at every statement of the block, after
//! the statement's moves and before its result is stored.

use crate::bitset::BitSet;
use crate::cfg::Cfg;
use crate::names;
use crate::paths::{Cover, MovePaths, PathId};
use crate::types::Types;
use crate::{Arg, Finding};
use sisu_diag::msg;
use sisu_sir::{
    BasicBlock, Callee, Function, LocalId, Operand, Place, Proj, Rvalue, Statement, StatementKind, Terminator,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub mi: BitSet,
    pub mu: BitSet,
}

impl State {
    fn bottom(n: usize) -> State {
        State { mi: BitSet::new(n), mu: BitSet::new(n) }
    }

    fn join(&mut self, o: &State) -> bool {
        let a = self.mi.union(&o.mi);
        let b = self.mu.union(&o.mu);
        a || b
    }
}

pub struct Analysis {
    pub paths: MovePaths,
    pub cfg: Cfg,
    pub entry: Vec<State>,
    pub reachable: Vec<bool>,
}

/// The operands a statement reads or moves, in evaluation order.
pub fn operands(s: &Statement) -> Vec<&Operand> {
    match &s.kind {
        StatementKind::Assign(_, rv) => rvalue_operands(rv),
        StatementKind::Call { callee, args, .. } => {
            let mut v: Vec<&Operand> = args.iter().collect();
            if let Callee::Value(o) = callee {
                v.push(o);
            }
            v
        }
        _ => Vec::new(),
    }
}

pub fn rvalue_operands(rv: &Rvalue) -> Vec<&Operand> {
    match rv {
        Rvalue::Use(o) | Rvalue::Unary(_, o) => vec![o],
        Rvalue::Binary(_, a, b) => vec![a, b],
        Rvalue::Aggregate(_, ops) | Rvalue::Closure(_, ops) => ops.iter().collect(),
        _ => Vec::new(),
    }
}

/// The destination a statement stores into, if any.
pub fn dest(s: &Statement) -> Option<&Place> {
    match &s.kind {
        StatementKind::Assign(p, _) | StatementKind::Call { dest: p, .. } => Some(p),
        _ => None,
    }
}

/// The locals named by index and key projections of a place.
pub fn index_locals(p: &Place) -> Vec<LocalId> {
    p.proj
        .iter()
        .filter_map(|x| match x {
            Proj::Index(l) | Proj::IndexFromEnd(l) | Proj::Key(l) => Some(*l),
            _ => None,
        })
        .collect()
}

impl Analysis {
    pub fn run(f: &Function, types: &Types) -> Analysis {
        let paths = MovePaths::build_with(f, types);
        let cfg = Cfg::new(f);
        let n = paths.len();
        let nb = f.blocks.len();
        let mut entry: Vec<State> = (0..nb).map(|_| State::bottom(n)).collect();
        let mut reachable = vec![false; nb];
        let mut a = Analysis { paths, cfg, entry: Vec::new(), reachable: Vec::new() };
        if nb == 0 {
            a.entry = entry;
            a.reachable = reachable;
            return a;
        }
        // At entry the parameters hold values; every other local is empty.
        let mut init = State::bottom(n);
        for l in 0..f.locals.len() {
            let root = a.paths.roots[l];
            if l >= 1 && l <= f.param_count as usize {
                a.set_init(&mut init, root);
            } else {
                a.set_uninit(&mut init, root);
            }
        }
        entry[0] = init;
        reachable[0] = true;
        let mut queued = vec![false; nb];
        let mut work: Vec<usize> = Vec::new();
        work.push(0);
        queued[0] = true;
        while let Some(b) = work.pop() {
            queued[b] = false;
            let block = &f.blocks[b];
            let mut st = entry[b].clone();
            let mut unwind_states: Vec<State> = Vec::new();
            for s in &block.stmts {
                a.apply_moves(f, s, &mut st);
                if block.unwind.is_some() {
                    unwind_states.push(st.clone());
                }
                a.apply_inits(s, &mut st, types, f);
            }
            if block.unwind.is_some() {
                unwind_states.push(st.clone());
            }
            let mut push = |t: usize, s: &State, entry: &mut Vec<State>, work: &mut Vec<usize>| {
                let changed = entry[t].join(s);
                if (changed || !reachable[t]) && !queued[t] {
                    reachable[t] = true;
                    queued[t] = true;
                    work.push(t);
                }
                reachable[t] = true;
            };
            if let Some(u) = block.unwind {
                for s in &unwind_states {
                    push(u.0 as usize, s, &mut entry, &mut work);
                }
            }
            for t in crate::cfg::targets(block) {
                if t >= nb {
                    continue;
                }
                let mut s = st.clone();
                a.edge_effect(f, types, block, t, &mut s);
                push(t, &s, &mut entry, &mut work);
            }
        }
        a.entry = entry;
        a.reachable = reachable;
        a
    }

    pub fn set_init(&self, st: &mut State, id: PathId) {
        for d in self.paths.descendants(id) {
            st.mi.insert(d);
            st.mu.remove(d);
        }
    }

    pub fn set_uninit(&self, st: &mut State, id: PathId) {
        for d in self.paths.descendants(id) {
            st.mi.remove(d);
            st.mu.insert(d);
        }
    }

    fn set_vacuous(&self, st: &mut State, id: PathId) {
        for d in self.paths.descendants(id) {
            st.mi.remove(d);
            st.mu.remove(d);
        }
    }

    /// The paths a statement moves out, drops or ends.
    pub fn removed_paths(&self, s: &Statement) -> Vec<PathId> {
        let mut out = Vec::new();
        for o in operands(s) {
            if let Operand::Move(p) = o
                && let (id, Cover::Exact) = self.paths.lookup(p)
            {
                out.push(id);
            }
        }
        match &s.kind {
            StatementKind::Drop { place, .. } => {
                if let (id, Cover::Exact) = self.paths.lookup(place) {
                    out.push(id);
                }
            }
            StatementKind::StorageDead(l) => out.push(self.paths.roots[l.0 as usize]),
            _ => {}
        }
        out
    }

    /// The path a statement initialises, if its destination is one.
    pub fn init_path(&self, s: &Statement) -> Option<PathId> {
        let d = dest(s)?;
        match self.paths.lookup(d) {
            (id, Cover::Exact) => Some(id),
            _ => None,
        }
    }

    pub fn apply_moves(&self, _f: &Function, s: &Statement, st: &mut State) {
        for id in self.removed_paths(s) {
            self.set_uninit(st, id);
        }
    }

    pub fn apply_inits(&self, s: &Statement, st: &mut State, _types: &Types, _f: &Function) {
        if let Some(id) = self.init_path(s) {
            self.set_init(st, id);
        }
    }

    pub fn apply(&self, f: &Function, types: &Types, s: &Statement, st: &mut State) {
        self.apply_moves(f, s, st);
        self.apply_inits(s, st, types, f);
    }

    /// What a switch on a discriminant teaches the edge to `target`.
    pub fn edge_effect(
        &self,
        f: &Function,
        types: &Types,
        block: &BasicBlock,
        target: usize,
        st: &mut State,
    ) {
        let Terminator::Switch { value: Operand::Copy(v), cases, default } = &block.term else { return };
        if !v.proj.is_empty() {
            return;
        }
        let Some(scrutinee) = block.stmts.iter().rev().find_map(|s| match &s.kind {
            StatementKind::Assign(p, rv) if *p == *v => match rv {
                Rvalue::Discriminant(q) => Some(Some(q)),
                _ => Some(None),
            },
            StatementKind::Call { dest, .. } if *dest == *v => Some(None),
            _ => None,
        }) else {
            return;
        };
        let Some(q) = scrutinee else { return };
        let (id, Cover::Exact) = self.paths.lookup(q) else { return };
        let Some(ty) = types.place_ty(f, q) else { return };
        let Some(count) = types.variant_count(&ty) else { return };
        let listed: Vec<i64> = cases.iter().map(|(k, _)| *k).collect();
        let mut active: Vec<i64> =
            cases.iter().filter(|(_, t)| t.0 as usize == target).map(|(k, _)| *k).collect();
        if default.0 as usize == target {
            active.extend((0..count as i64).filter(|k| !listed.contains(k)));
        }
        for c in self.paths.paths[id].children.clone() {
            if let Some(Proj::Downcast(var)) = self.paths.paths[c].place.proj.last()
                && !active.contains(&(*var as i64))
            {
                self.set_vacuous(st, c);
            }
        }
    }

    /// The state before each statement of a block, and at its terminator.
    pub fn states(&self, f: &Function, types: &Types, b: usize) -> Vec<State> {
        let mut out = Vec::new();
        let mut st = self.entry[b].clone();
        for s in &f.blocks[b].stmts {
            out.push(st.clone());
            self.apply(f, types, s, &mut st);
        }
        out.push(st);
        out
    }

    /// Whether the path and everything below it are initialised.
    pub fn fully_init(&self, st: &State, id: PathId) -> bool {
        self.paths.descendants(id).iter().all(|d| !st.mu.contains(*d))
    }

    /// Whether the path and everything below it are uninitialised.
    pub fn fully_uninit(&self, st: &State, id: PathId) -> bool {
        self.paths.descendants(id).iter().all(|d| !st.mi.contains(*d))
    }

    // ---------------------------------------------------------------- diagnostics

    pub fn findings(&self, f: &Function, types: &Types) -> Vec<Finding> {
        let mut out = Vec::new();
        let mut reported: Vec<(PathId, Option<(usize, usize)>)> = Vec::new();
        for (b, block) in f.blocks.iter().enumerate() {
            if !self.reachable[b] || block.cleanup {
                continue;
            }
            let states = self.states(f, types, b);
            for (si, s) in block.stmts.iter().enumerate() {
                let st = &states[si];
                // Each use, and whether it needs the whole value (`true`) or
                // only the path itself and what contains it (`false`).
                let mut uses: Vec<(Place, bool)> = Vec::new();
                for o in operands(s) {
                    if let Operand::Copy(p) | Operand::Move(p) = o {
                        uses.push((p.clone(), true));
                    }
                }
                if let StatementKind::Assign(_, rv) = &s.kind
                    && let Rvalue::Ref(p, _)
                    | Rvalue::Discriminant(p)
                    | Rvalue::CopyOf(p)
                    | Rvalue::StrView(p) = rv
                {
                    uses.push((p.clone(), true));
                }
                let mut extra: Vec<(Place, bool)> = Vec::new();
                for (p, _) in uses.iter() {
                    extra.extend(index_locals(p).into_iter().map(|l| (Place::local(l), true)));
                }
                if let Some(d) = dest(s) {
                    extra.extend(index_locals(d).into_iter().map(|l| (Place::local(l), true)));
                    match self.paths.lookup(d) {
                        (_, Cover::Exact) => {
                            // Assigning a field needs the value around it.
                            if !d.proj.is_empty() {
                                let parent =
                                    Place { local: d.local, proj: d.proj[..d.proj.len() - 1].to_vec() };
                                extra.push((strip_downcast(parent), false));
                            }
                        }
                        (_, Cover::Through) | (_, Cover::Field) => {
                            extra.push((base_before_through(d), false))
                        }
                    }
                }
                for (p, whole) in uses.iter().chain(extra.iter()) {
                    if let Some(fd) = self.check_use(f, types, b, si, st, p, *whole, &mut reported) {
                        out.push(fd);
                    }
                }
                // A `let` declared without a value is assigned once per path.
                if let Some(d) = dest(s)
                    && d.proj.is_empty()
                    && f.locals.get(d.local.0 as usize).is_some_and(|l| l.once)
                    && st.mi.contains(self.paths.roots[d.local.0 as usize])
                    && !reported.contains(&(self.paths.roots[d.local.0 as usize], None))
                {
                    reported.push((self.paths.roots[d.local.0 as usize], None));
                    let name = names::local_name(f, d.local);
                    let first = self.find_site(f, b, si, &[self.paths.roots[d.local.0 as usize]], Site::Init);
                    let mut fd =
                        Finding::new(msg::TYPE_REASSIGN_LET, f, s.loc).with("name", name.clone()).at(b, si);
                    if let Some((fb, fs)) = first {
                        // The same statement, run again by a loop.
                        let again = (fb, fs) == (b, si);
                        let label =
                            if again { msg::BORROW_ASSIGNED_IN_LOOP } else { msg::BORROW_ASSIGNED_HERE };
                        fd = fd.label(f.blocks[fb].stmts[fs].loc, label, vec![("name", Arg::Text(name))]);
                    }
                    out.push(fd);
                }
                // Moving out of a value that implements `Drop`, or through a
                // reference or an index.
                for o in operands(s) {
                    if let Operand::Move(p) = o
                        && let Some(fd) = self.check_move_out(f, types, b, si, s, p)
                    {
                        out.push(fd);
                    }
                }
            }
            // The terminator's operand.
            if let Terminator::Switch { value: Operand::Copy(p) | Operand::Move(p), .. } = &block.term {
                let st = &states[block.stmts.len()];
                let si = block.stmts.len();
                if let Some(fd) = self.check_use(f, types, b, si, st, p, true, &mut reported) {
                    out.push(fd);
                }
            }
        }
        out
    }

    fn check_move_out(
        &self,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        s: &Statement,
        p: &Place,
    ) -> Option<Finding> {
        if p.proj
            .iter()
            .any(|x| matches!(x, Proj::Deref | Proj::Index(_) | Proj::IndexFromEnd(_) | Proj::Key(_)))
        {
            let name = names::place_name(types, f, p);
            let mut fd =
                Finding::new(msg::BORROW_MOVE_OUT_OF_PLACE, f, s.loc).with_arg("name", name).at(b, si);
            // The value it would come out of.
            if names::is_named(f, p.local) {
                let root = names::local_name(f, p.local);
                let decl = f.locals.get(p.local.0 as usize).and_then(|l| l.loc);
                fd = fd.label(decl, msg::BORROW_DECLARED_HERE, vec![("name", Arg::Text(root))]);
            } else if let Some(made) = names::temp_source(f, p.local) {
                fd = fd.label(Some(made), msg::BORROW_MADE_HERE, Vec::new());
            }
            return Some(fd);
        }
        // Each prefix ending before a field: does its type implement Drop?
        for i in 0..p.proj.len() {
            if !matches!(p.proj[i], Proj::Field(_)) {
                continue;
            }
            let base = strip_downcast(Place { local: p.local, proj: p.proj[..i].to_vec() });
            let Some(t) = types.place_ty(f, &base) else { continue };
            if types.implements_drop(&t) {
                let ty = sisu_sir::write::ty(&t);
                return Some(
                    Finding::new(msg::BORROW_MOVE_OUT_OF_DROP, f, s.loc)
                        .with_arg("part", names::place_name(types, f, p))
                        .with_arg("name", names::place_name(types, f, &base))
                        .with("ty", ty.clone())
                        .label(
                            types.drop_impl_loc(&t),
                            msg::BORROW_DROP_IMPL_HERE,
                            vec![("ty", Arg::Text(ty))],
                        )
                        .at(b, si),
                );
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn check_use(
        &self,
        f: &Function,
        types: &Types,
        b: usize,
        si: usize,
        st: &State,
        p: &Place,
        whole: bool,
        reported: &mut Vec<(PathId, Option<(usize, usize)>)>,
    ) -> Option<Finding> {
        let (id, cover) = self.paths.lookup(p);
        let loc = f.blocks[b].stmts.get(si).and_then(|s| s.loc).or(f.blocks[b].loc);
        if st.mu.contains(id) {
            let site = self.find_site(f, b, si, &self.related(id), Site::Move);
            let key = (self.paths.roots[self.paths.local_of(id).0 as usize], site);
            if reported.contains(&key) {
                return None;
            }
            reported.push(key);
            let used = names::place_name(types, f, &self.paths.paths[id].place);
            return Some(match site {
                Some((mb, ms)) => {
                    let mloc = f.blocks[mb].stmts[ms].loc;
                    let line = mloc.map(|l| l.line).unwrap_or(0);
                    // A move at or after the use was reached around a loop.
                    let later = (mb, ms) == (b, si)
                        || match (mloc, loc) {
                            (Some(m), Some(u)) => (m.line, m.col) >= (u.line, u.col),
                            _ => false,
                        };
                    let label = if later { msg::BORROW_MOVED_IN_LOOP } else { msg::BORROW_MOVED_HERE };
                    Finding::new(msg::BORROW_USE_AFTER_MOVE, f, loc)
                        .with_arg("name", used.clone())
                        .with("line", line.to_string())
                        .label(mloc, label, vec![("name", used)])
                        .note(msg::BORROW_COPY_BEFORE_MOVE, Vec::new())
                        .at(b, si)
                }
                None => {
                    let decl = f.locals.get(p.local.0 as usize).and_then(|l| l.loc);
                    let mut fd = Finding::new(msg::TYPE_UNINITIALIZED, f, loc)
                        .with_arg("name", used.clone())
                        .at(b, si);
                    if decl.is_some() {
                        fd = fd.label(decl, msg::BORROW_DECLARED_HERE, vec![("name", used)]);
                    }
                    fd
                }
            });
        }
        if whole && matches!(cover, Cover::Exact | Cover::Through) {
            for d in self.paths.descendants(id).into_iter().skip(1) {
                if st.mu.contains(d) {
                    let site = self.find_site(f, b, si, &self.paths.descendants(d), Site::Move);
                    let key = (d, site);
                    if reported.contains(&key) {
                        return None;
                    }
                    reported.push(key);
                    let whole = names::place_name(types, f, &self.paths.paths[id].place);
                    let part = names::place_name(types, f, &self.paths.paths[d].place);
                    let mloc = site.and_then(|(mb, ms)| f.blocks[mb].stmts[ms].loc);
                    return Some(
                        Finding::new(msg::BORROW_USE_AFTER_PARTIAL_MOVE, f, loc)
                            .with_arg("name", whole)
                            .with_arg("part", part.clone())
                            .with("line", mloc.map(|l| l.line).unwrap_or(0).to_string())
                            .label(mloc, msg::BORROW_MOVED_HERE, vec![("name", part)])
                            .at(b, si),
                    );
                }
            }
        }
        None
    }

    /// The paths whose move makes `id` uninitialised: itself, its
    /// ancestors and its descendants.
    fn related(&self, id: PathId) -> Vec<PathId> {
        let mut v = self.paths.descendants(id);
        v.extend(self.paths.ancestors(id));
        v
    }

    /// The nearest statement before (block, stmt) that moves (or, for
    /// `Site::Init`, initialises) one of `targets`, searching backwards
    /// and stopping at statements that initialise the path again.
    pub fn find_site(
        &self,
        f: &Function,
        b: usize,
        si: usize,
        targets: &[PathId],
        what: Site,
    ) -> Option<(usize, usize)> {
        let mut visited = vec![false; f.blocks.len()];
        let mut queue: std::collections::VecDeque<(usize, usize)> = std::collections::VecDeque::new();
        queue.push_back((b, si));
        let mut first = true;
        while let Some((blk, end)) = queue.pop_front() {
            if !first {
                if visited[blk] {
                    continue;
                }
                visited[blk] = true;
            }
            first = false;
            let stmts = &f.blocks[blk].stmts;
            let mut stopped = false;
            for i in (0..end.min(stmts.len())).rev() {
                let s = &stmts[i];
                match what {
                    Site::Move => {
                        if self.removed_paths(s).iter().any(|p| targets.contains(p))
                            && !matches!(s.kind, StatementKind::StorageDead(_))
                        {
                            return Some((blk, i));
                        }
                        if self.init_path(s).is_some_and(|p| targets.contains(&p)) {
                            stopped = true;
                            break;
                        }
                    }
                    Site::Init => {
                        if self.init_path(s).is_some_and(|p| targets.contains(&p)) {
                            return Some((blk, i));
                        }
                    }
                }
            }
            if stopped {
                continue;
            }
            for &p in &self.cfg.pred[blk] {
                if !f.blocks[p].cleanup && !visited[p] {
                    queue.push_back((p, f.blocks[p].stmts.len()));
                }
            }
        }
        None
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Site {
    Move,
    Init,
}

fn strip_downcast(mut p: Place) -> Place {
    while matches!(p.proj.last(), Some(Proj::Downcast(_))) {
        p.proj.pop();
    }
    p
}

/// The part of a place before its first deref, index or key.
fn base_before_through(p: &Place) -> Place {
    let n =
        p.proj.iter().position(|x| !matches!(x, Proj::Field(_) | Proj::Downcast(_))).unwrap_or(p.proj.len());
    let n = if n == p.proj.len() { n.saturating_sub(1) } else { n };
    strip_downcast(Place { local: p.local, proj: p.proj[..n].to_vec() })
}
